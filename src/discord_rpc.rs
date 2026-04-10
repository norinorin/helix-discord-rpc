use discord_rich_presence::{
    DiscordIpc, DiscordIpcClient,
    activity::{self, Activity, Assets},
};
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, ListenerOptions, Name, ToFsName, prelude::*,
};
use rkyv::{Archive, Deserialize, Serialize, rancor::Error};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;
use steel::rvals::Custom;

use crate::assets::{HELIX_ICON_URL, IDLE_ICON_URL, get_asset_url};

const SOCKET_BASE_NAME: &str = "helix-discord-rpc.sock";
const DISCORD_APP_ID: &str = "1491839617421414581";

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
pub enum RpcCommand {
    SetActivity {
        path: String,
        details: String,
        state: String,
    },
    SetIdle,
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

    pub fn set_activity(&mut self, path: String, details: String, state: String) {
        let _ = self.tx.send(RpcCommand::SetActivity {
            path,
            details,
            state,
        });
    }

    pub fn set_idle(&mut self) {
        let _ = self.tx.send(RpcCommand::SetIdle);
    }

    fn background_loop(rx: Receiver<RpcCommand>, main_tx: Sender<RpcCommand>) {
        loop {
            let name = Self::get_socket_name();
            match ListenerOptions::new().name(name.clone()).create_sync() {
                Ok(listener) => Self::run_primary(listener, &rx, &main_tx),
                Err(_) => Self::run_replica(name, &rx, &main_tx),
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

        Self::spawn_connection_acceptor(listener, main_tx.clone());

        while let Ok(mut cmd) = rx.recv() {
            while let Ok(next_cmd) = rx.try_recv() {
                cmd = next_cmd;
            }

            if !is_connected && last_connect_attempt.elapsed() >= Duration::from_secs(30) {
                ipc_client = DiscordIpcClient::new(DISCORD_APP_ID).unwrap();
                is_connected = ipc_client.connect().is_ok();
                last_connect_attempt = std::time::Instant::now();
            }

            if is_connected {
                is_connected = match cmd {
                    RpcCommand::SetActivity {
                        path,
                        details,
                        state,
                    } => Self::handle_set_activity(&mut ipc_client, path, details, state),
                    RpcCommand::SetIdle => Self::handle_set_idle(&mut ipc_client),
                };
                thread::sleep(Duration::from_secs(3));
            }
        }
    }

    fn run_replica(name: Name<'_>, rx: &Receiver<RpcCommand>, main_tx: &Sender<RpcCommand>) {
        let mut last_cmd = Option::<RpcCommand>::None;

        if let Ok(mut stream) = LocalSocketStream::connect(name) {
            while let Ok(cmd) = rx.recv() {
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

            if let Some(cmd) = last_cmd {
                let _ = main_tx.send(cmd);
            }
        } else {
            if !GenericNamespaced::is_supported() {
                let path = std::env::temp_dir().join(SOCKET_BASE_NAME);
                let _ = std::fs::remove_file(path);
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn spawn_connection_acceptor(listener: LocalSocketListener, main_tx: Sender<RpcCommand>) {
        thread::spawn(move || {
            for mut stream in listener.incoming().filter_map(Result::ok) {
                let stream_tx = main_tx.clone();
                thread::spawn(move || {
                    loop {
                        let mut len_buf = [0u8; 4];
                        if stream.read_exact(&mut len_buf).is_err() {
                            break;
                        }

                        let len = u32::from_le_bytes(len_buf) as usize;
                        let mut payload = vec![0u8; len];
                        if stream.read_exact(&mut payload).is_err() {
                            break;
                        }

                        if let Ok(cmd) = rkyv::from_bytes::<RpcCommand, Error>(&payload) {
                            let _ = stream_tx.send(cmd);
                        }
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
    ) -> bool {
        let filename = Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown");
        let asset_url = get_asset_url(filename);
        let activity = activity::Activity::new()
            .state(&state)
            .details(&details)
            .assets(
                Assets::new()
                    .large_image(&asset_url)
                    .small_image(HELIX_ICON_URL)
                    .small_text("A post-modern modal text editor"),
            );

        ipc_client.set_activity(activity).is_ok()
    }

    fn handle_set_idle(ipc_client: &mut DiscordIpcClient) -> bool {
        let activity = Activity::new().assets(Assets::new().small_image(IDLE_ICON_URL));
        ipc_client.set_activity(activity).is_ok()
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
}
