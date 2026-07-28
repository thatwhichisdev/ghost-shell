use std::{
    env::{self},
    path::PathBuf,
};

use anyhow::Result;
use ghost_shell_ipc::{
    protocol::{Request, Response},
    server::{AsyncRequest, Server},
};
use ghost_shell_niri::client::client::NiriClient;
use gpui::{App, accesskit::Uuid, prelude::*};
use gpui_tokio::Tokio;
use tokio::sync::mpsc;

pub struct AppState;

impl AppState {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// Loads app configuration and opens bars on available displays.
///
/// # Errors
/// Bubbles up errors from bar initialization
///
pub fn init(cx: &mut App) -> Result<()> {
    let config = ghost_shell_config::load()
        .inspect_err(|e| eprintln!("Failed to load config {e:?}"))
        .unwrap_or_default();

    let niri_ipc_client =
        Tokio::handle(cx).block_on(NiriClient::new()).unwrap();
    let niri_event_reader =
        Tokio::handle(cx).block_on(niri_ipc_client.into_event_reader());
    let niri_event_receiver = niri_event_reader.subscribe();

    let (request_sender, mut request_receiver) =
        mpsc::channel::<AsyncRequest>(32);
    let ipc_socket_path =
        env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap();
    let ipc_server = Tokio::handle(cx).block_on(Server::bind(
        ipc_socket_path.join("ghost-shell-daemon"),
        request_sender,
    ))?;

    let menu_widget = cx.new(|_cx| ghost_shell_system::menu::Menu {});
    let battery_widget = cx.new(|_cx| ghost_shell_power::battery::Battery {});
    let clock_widget = cx.new(|cx| {
        ghost_shell_system::clock::Clock::new(config.clock.clone(), cx)
    });
    let focus_widget = cx
        .new(|cx| ghost_shell_niri::focus::Focus::new(cx, niri_event_receiver));

    for (output_name, bar_config) in config.bars {
        if let Some(display) = cx.displays().into_iter().find(|display| {
            let output_uuid =
                Uuid::new_v5(&Uuid::NAMESPACE_DNS, output_name.as_bytes());
            display.uuid().is_ok_and(|uuid| uuid == output_uuid)
        }) {
            let niri_event_receiver = niri_event_reader.subscribe();
            let workspaces_widget = cx.new(|cx| {
                ghost_shell_niri::workspaces::Workspaces::new(
                    cx,
                    output_name,
                    niri_event_receiver,
                )
            });

            ghost_shell_bar::open(
                &display,
                config.general.clone(),
                bar_config,
                menu_widget.clone(),
                workspaces_widget,
                focus_widget.clone(),
                battery_widget.clone(),
                clock_widget.clone(),
                cx,
            )?;
        }
    }

    Tokio::spawn(cx, niri_event_reader.run()).detach();
    Tokio::spawn(cx, ipc_server.run()).detach();

    // Spawn task to handle request over IPC connection
    cx.spawn(async move |cx| {
        while let Some(incoming) = request_receiver.recv().await {
            let reply: std::result::Result<Response, String> =
                cx.update(|_cx| match incoming.request {
                    Request::Launcher { action } => {
                        println!("Launcher action: {action:?}");

                        Ok(Response::Handled)
                    }
                });

            let _ = incoming.reply.send(reply);
        }
    })
    .detach();

    Ok(())
}
