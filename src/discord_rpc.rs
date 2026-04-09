use discord_rich_presence::{
    DiscordIpc, DiscordIpcClient,
    activity::{self, Activity, Assets},
};
use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;
use steel::rvals::Custom;

use crate::assets::get_asset;

pub enum RpcCommand {
    Connect,
    SetActivity {
        path: String,
        workspace: String,
        row: usize,
        col: usize,
    },
    SetIdle,
}

pub struct DiscordRPC {
    tx: Sender<RpcCommand>,
}

impl Custom for DiscordRPC {}

/// Implementation of the functionnalities we need to use in steel
impl DiscordRPC {
    /// Creates and return the "server" as it is called in the steel module
    /// This object stays alive as long as it is assigned to a steel variable
    pub fn new() -> DiscordRPC {
        let (tx, rx) = mpsc::channel::<RpcCommand>();
        thread::spawn(move || {
            let mut ipc_client = DiscordIpcClient::new("1173288863217766421").unwrap();
            let mut connected = false;

            // Block until at least one command is received
            while let Ok(mut cmd) = rx.recv() {
                // Drain the queue to keep only the most recent command.
                // This clears out stale keystrokes that queued up while we were sleeping.
                while let Ok(next_cmd) = rx.try_recv() {
                    cmd = next_cmd;
                }

                match cmd {
                    RpcCommand::Connect => {
                        let _ = ipc_client.connect();
                        connected = true;
                    }
                    RpcCommand::SetActivity {
                        path,
                        workspace,
                        row,
                        col,
                    } => {
                        if connected {
                            let filename = Path::new(&path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Unknown");

                            let state =
                                format!("Editing {} {}:{}", filename, row, col.saturating_sub(6));

                            let folder = Path::new(&workspace)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Unknown");
                            let details = format!("Workspace {}", folder);

                            let asset_name = get_asset(filename.to_string());
                            let activity = activity::Activity::new()
                                .state(&state)
                                .details(&details)
                                .assets(
                                    Assets::new()
                                        .large_image(&asset_name)
                                        .small_image("helix")
                                        .small_text("https://github.com/helix-editor/helix"),
                                );

                            let _ = ipc_client.set_activity(activity);

                            // Throttle window: block further dispatches for 3 seconds
                            thread::sleep(Duration::from_secs(3));
                        }
                    }
                    RpcCommand::SetIdle => {
                        if connected {
                            let activity =
                                Activity::new().assets(Assets::new().small_image("idle"));
                            let _ = ipc_client.set_activity(activity);
                            thread::sleep(Duration::from_secs(3));
                        }
                    }
                }
            }
        });

        DiscordRPC { tx }
    }

    pub fn connect(&mut self) {
        let _ = self.tx.send(RpcCommand::Connect);
    }

    pub fn set_activity(&mut self, path: String, workspace: String, row: usize, col: usize) {
        let _ = self.tx.send(RpcCommand::SetActivity {
            path,
            workspace,
            row,
            col,
        });
    }

    pub fn set_idle(&mut self) {
        let _ = self.tx.send(RpcCommand::SetIdle);
    }
}
