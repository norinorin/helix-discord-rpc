use discord_rich_presence::{
    DiscordIpc, DiscordIpcClient,
    activity::{self, Activity, Assets, Timestamps},
};
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, ListenerOptions, Name, ToFsName, prelude::*,
};
use rkyv::{Archive, Deserialize, Serialize, rancor::Error};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;
use std::{collections::HashMap, path::Path};
use std::{
    io::{Read, Write},
    time::{SystemTime, UNIX_EPOCH},
};
use steel::rvals::Custom;

use crate::assets::{HELIX_ICON_URL, IDLE_ICON_URL, get_asset_url};

const SOCKET_BASE_NAME: &str = "helix-discord-rpc.sock";
const DISCORD_APP_ID: &str = "1491839617421414581";
const MAX_PAYLOAD_SIZE: usize = 64 * 1024;
const MIN_UPDATE_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
pub enum RpcCommand {
    SetActivity {
        pid: u32,
        path: String,
        workspace: String,
        details: String,
        state: String,
    },
    SetIdle {
        pid: u32,
    },
    ClearActivity {
        pid: u32,
    },
    SetResetMode(String),
}

#[derive(Clone, Debug)]
struct InstanceActivity {
    path: String,
    workspace: String,
    details: String,
    state: String,
    updated_at: std::time::Instant,
}

pub struct DiscordRPC {
    tx: Sender<RpcCommand>,
}

impl Custom for DiscordRPC {}

impl DiscordRPC {
    pub fn new() -> DiscordRPC {
        let (tx, rx) = mpsc::channel::<RpcCommand>();
        let main_tx = tx.clone();

        thread::spawn(move || Self::background_loop(rx, main_tx));

        DiscordRPC { tx }
    }

    pub fn set_activity(
        &mut self,
        path: String,
        workspace: String,
        details: String,
        state: String,
    ) {
        let _ = self.tx.send(RpcCommand::SetActivity {
            pid: std::process::id(),
            path,
            workspace,
            details,
            state,
        });
    }

    pub fn set_idle(&mut self) {
        let _ = self.tx.send(RpcCommand::SetIdle {
            pid: std::process::id(),
        });
    }

    pub fn clear_activity(&mut self) {
        let _ = self.tx.send(RpcCommand::ClearActivity {
            pid: std::process::id(),
        });
    }

    pub fn set_reset_mode(&mut self, mode: String) {
        let _ = self.tx.send(RpcCommand::SetResetMode(mode));
    }

    fn background_loop(rx: Receiver<RpcCommand>, main_tx: Sender<RpcCommand>) {
        let mut last_cmd = None;
        loop {
            let name = Self::get_socket_name();

            let listener = ListenerOptions::new()
                .name(name.clone())
                .create_sync()
                .or_else(|_| {
                    if LocalSocketStream::connect(name.clone()).is_err() {
                        Self::cleanup_stale_socket();
                        ListenerOptions::new().name(name.clone()).create_sync()
                    } else {
                        Err(std::io::Error::from(std::io::ErrorKind::AddrInUse))
                    }
                });

            match listener {
                Ok(listener) => {
                    if let Some(cmd) = last_cmd.take() {
                        let _ = main_tx.send(cmd);
                    }
                    Self::run_primary(listener, &rx, &main_tx);
                }
                Err(_) => {
                    last_cmd = Self::run_replica(name, &rx, last_cmd);
                }
            }
        }
    }

    fn update_instance_state(
        active_instances: &mut HashMap<u32, InstanceActivity>,
        reset_mode: &mut String,
        cmd: RpcCommand,
    ) {
        match cmd {
            RpcCommand::SetResetMode(mode) => {
                *reset_mode = mode;
            }
            RpcCommand::SetActivity {
                pid,
                path,
                workspace,
                details,
                state,
            } => {
                active_instances.insert(
                    pid,
                    InstanceActivity {
                        path,
                        workspace,
                        details,
                        state,
                        updated_at: std::time::Instant::now(),
                    },
                );
            }
            RpcCommand::ClearActivity { pid } | RpcCommand::SetIdle { pid } => {
                active_instances.remove(&pid);
            }
        }
    }

    fn run_primary(
        listener: LocalSocketListener,
        rx: &Receiver<RpcCommand>,
        main_tx: &Sender<RpcCommand>,
    ) {
        let mut ipc_client = DiscordIpcClient::new(DISCORD_APP_ID).unwrap();
        let mut is_connected = false;
        let mut last_connect_attempt = std::time::Instant::now() - Duration::from_secs(30);
        let mut last_update = std::time::Instant::now() - MIN_UPDATE_INTERVAL;

        let mut active_instances: HashMap<u32, InstanceActivity> = HashMap::new();
        let mut current_active_pid: Option<u32> = None;

        let session_start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let mut current_start = session_start;
        let mut reset_mode = String::from("workspace");
        let mut prev_path = String::new();
        let mut prev_workspace = String::new();
        let mut prev_details = String::new();
        let mut prev_state = String::new();

        Self::spawn_connection_acceptor(listener, main_tx.clone());

        while let Ok(cmd) = rx.recv() {
            Self::update_instance_state(&mut active_instances, &mut reset_mode, cmd);
            while let Ok(next_cmd) = rx.try_recv() {
                Self::update_instance_state(&mut active_instances, &mut reset_mode, next_cmd);
            }

            if !is_connected && last_connect_attempt.elapsed() >= Duration::from_secs(30) {
                ipc_client = DiscordIpcClient::new(DISCORD_APP_ID).unwrap();
                is_connected = ipc_client.connect().is_ok();
                last_connect_attempt = std::time::Instant::now();
            }

            if is_connected {
                let elapsed = last_update.elapsed();
                if elapsed < MIN_UPDATE_INTERVAL {
                    thread::sleep(MIN_UPDATE_INTERVAL - elapsed);
                }

                let most_recent = active_instances
                    .iter()
                    .max_by_key(|(_, act)| act.updated_at);

                if let Some((&pid, act)) = most_recent {
                    let instance_switched = current_active_pid != Some(pid);
                    let should_reset = instance_switched
                        || match reset_mode.as_str() {
                            "file" => act.path != prev_path,
                            "workspace" => act.workspace != prev_workspace,
                            "never" => false,
                            _ => {
                                act.path != prev_path
                                    || act.workspace != prev_workspace
                                    || act.details != prev_details
                                    || act.state != prev_state
                            }
                        };

                    if should_reset {
                        current_start = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);
                    }

                    prev_path = act.path.clone();
                    prev_workspace = act.workspace.clone();
                    prev_details = act.details.clone();
                    prev_state = act.state.clone();
                    current_active_pid = Some(pid);

                    if Self::handle_set_activity(
                        &mut ipc_client,
                        act.path.clone(),
                        act.details.clone(),
                        act.state.clone(),
                        current_start,
                    ) {
                        last_update = std::time::Instant::now();
                    } else {
                        is_connected = false;
                    }
                } else {
                    if current_active_pid.is_some() {
                        if Self::handle_clear_activity(&mut ipc_client) {
                            current_active_pid = None;
                            last_update = std::time::Instant::now();
                        } else {
                            is_connected = false;
                        }
                    }
                }
            }
        }
    }

    fn run_replica(
        name: Name<'_>,
        rx: &Receiver<RpcCommand>,
        mut last_cmd: Option<RpcCommand>,
    ) -> Option<RpcCommand> {
        let Ok(mut stream) = LocalSocketStream::connect(name) else {
            thread::sleep(Duration::from_millis(200));
            return last_cmd;
        };

        let _ = stream.set_nonblocking(true);
        let mut buf = [0u8; 1];

        if let Some(ref cmd) = last_cmd
            && let Ok(bytes) = rkyv::to_bytes::<Error>(cmd)
        {
            let len_bytes = (bytes.len() as u32).to_le_bytes();
            if stream.write_all(&len_bytes).is_ok() {
                let _ = stream.write_all(&bytes);
            }
        }

        loop {
            match stream.read(&mut buf) {
                Ok(0) => break, // eof
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => break,
            }

            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(cmd) => {
                    last_cmd = Some(cmd.clone());

                    let bytes = match rkyv::to_bytes::<Error>(&cmd) {
                        Ok(b) => b,
                        Err(_) => continue,
                    };

                    let len_bytes = (bytes.len() as u32).to_le_bytes();
                    if stream.write_all(&len_bytes).is_err() || stream.write_all(&bytes).is_err() {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        last_cmd
    }

    fn spawn_connection_acceptor(listener: LocalSocketListener, main_tx: Sender<RpcCommand>) {
        thread::spawn(move || {
            for mut stream in listener.incoming().filter_map(Result::ok) {
                let stream_tx = main_tx.clone();
                thread::spawn(move || {
                    let mut last_pid = None;

                    loop {
                        let mut len_buf = [0u8; 4];
                        if stream.read_exact(&mut len_buf).is_err() {
                            break;
                        }

                        let len = u32::from_le_bytes(len_buf) as usize;
                        if len == 0 || len > MAX_PAYLOAD_SIZE {
                            break;
                        }

                        let mut payload = vec![0u8; len];
                        if stream.read_exact(&mut payload).is_err() {
                            break;
                        }

                        if let Ok(cmd) = rkyv::from_bytes::<RpcCommand, Error>(&payload) {
                            match &cmd {
                                RpcCommand::SetActivity { pid, .. }
                                | RpcCommand::SetIdle { pid }
                                | RpcCommand::ClearActivity { pid } => {
                                    last_pid = Some(*pid);
                                }
                                _ => {}
                            }
                            let _ = stream_tx.send(cmd);
                        }
                    }

                    // replica dies
                    if let Some(pid) = last_pid {
                        let _ = stream_tx.send(RpcCommand::ClearActivity { pid });
                    }
                });
            }
        });
    }

    fn handle_set_activity(
        ipc_client: &mut DiscordIpcClient,
        path: String,
        details: String,
        state: String,
        start_timestamp: i64,
    ) -> bool {
        let filename = Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown");
        let asset_url = get_asset_url(filename);
        let activity = activity::Activity::new()
            .state(&state)
            .details(&details)
            .timestamps(Timestamps::new().start(start_timestamp))
            .assets(
                Assets::new()
                    .large_image(&asset_url)
                    .small_image(HELIX_ICON_URL)
                    .small_text("A post-modern modal text editor"),
            );

        ipc_client.set_activity(activity).is_ok()
    }

    #[allow(dead_code)]
    fn handle_set_idle(ipc_client: &mut DiscordIpcClient) -> bool {
        let activity = Activity::new().assets(Assets::new().small_image(IDLE_ICON_URL));
        ipc_client.set_activity(activity).is_ok()
    }

    fn handle_clear_activity(ipc_client: &mut DiscordIpcClient) -> bool {
        ipc_client.clear_activity().is_ok()
    }

    fn get_socket_name<'a>() -> Name<'a> {
        if GenericNamespaced::is_supported() {
            SOCKET_BASE_NAME.to_ns_name::<GenericNamespaced>().unwrap()
        } else {
            std::env::temp_dir()
                .join(SOCKET_BASE_NAME)
                .to_string_lossy()
                .to_string()
                .to_fs_name::<GenericFilePath>()
                .unwrap()
        }
    }

    fn cleanup_stale_socket() {
        if !GenericNamespaced::is_supported() {
            let path = std::env::temp_dir().join(SOCKET_BASE_NAME);
            let _ = std::fs::remove_file(path);
        }
    }
}
