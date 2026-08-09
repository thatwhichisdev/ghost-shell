pub mod client;
pub mod protocol;
pub mod server;

pub use client::*;
pub use protocol::*;
pub use server::*;

use std::{env, path::PathBuf};

use gpui::App;
use tokio::sync::mpsc::{self};

use ghost_shell_actions::{
    FinderClose, FinderOpen, FinderToggle, LauncherClose, LauncherOpen,
    LauncherToggle,
};

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

            cx.update(|cx| match request {
                Request::Launcher { action } => match action {
                    LauncherAction::Open => cx.dispatch_action(&LauncherOpen),
                    LauncherAction::Close => cx.dispatch_action(&LauncherClose),
                    LauncherAction::Toggle => {
                        cx.dispatch_action(&LauncherToggle)
                    }
                },
                Request::Finder { action } => match action {
                    FinderAction::Open => cx.dispatch_action(&FinderOpen),
                    FinderAction::Close => cx.dispatch_action(&FinderClose),
                    FinderAction::Toggle => cx.dispatch_action(&FinderToggle),
                },
            });

            let _ = reply.send(Ok(Response::Handled));
        }
    })
    .detach();
}
