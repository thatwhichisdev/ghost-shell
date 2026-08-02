pub mod client;
pub mod protocol;
pub mod server;

pub use client::*;
pub use protocol::*;
pub use server::*;

use std::{env, path::PathBuf};

use gpui::App;
use tokio::sync::mpsc::{self};

use ghost_shell_actions::ToggleLauncher;

pub fn init(cx: &mut App) {
    let (sender, mut receiver) = mpsc::channel::<AsyncRequest>(256);

    let socket_path = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap()
        .join("ghost-shell-daemon");

    let server = gpui_tokio::Tokio::handle(cx)
        .block_on(Server::bind(socket_path, sender))
        .unwrap();

    gpui_tokio::Tokio::spawn(cx, server.run()).detach();

    cx.spawn(async move |cx| {
        while let Some(request) = receiver.recv().await {
            let AsyncRequest { request, reply } = request;

            cx.update(|cx| {
                let action = match request {
                    Request::Launcher {
                        action: LauncherAction::Toggle,
                    } => ToggleLauncher,
                };
                cx.dispatch_action(&action);
            });

            let _ = reply.send(Ok(Response::Handled));
        }
    })
    .detach();
}
