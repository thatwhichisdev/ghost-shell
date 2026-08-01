pub mod client;
pub mod protocol;
pub mod server;

pub use client::*;
pub use protocol::*;
pub use server::*;

use gpui::{App, Global};
use std::{env, path::PathBuf};
use tokio::sync::mpsc::{self, Receiver};

pub struct RequestHandler {
    pub receiver: Receiver<AsyncRequest>,
}

impl Global for RequestHandler {}

pub fn init(cx: &mut App) {
    let (sender, receiver) = mpsc::channel::<AsyncRequest>(256);

    let socket_path = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap()
        .join("ghost-shell-daemon");

    let server = Server::bind(socket_path, sender).unwrap();

    gpui_tokio::Tokio::spawn(cx, server.run()).detach();

    let request_handler = RequestHandler { receiver };
    cx.set_global(request_handler);
}
