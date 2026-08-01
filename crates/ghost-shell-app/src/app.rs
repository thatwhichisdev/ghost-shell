use std::{
    env::{self},
    path::PathBuf,
};

use anyhow::Result;
use gpui::{App, accesskit::Uuid, prelude::*};
use gpui_tokio::Tokio;
use tokio::sync::mpsc;

use ghost_shell_ipc::{
    protocol::{LauncherAction, Request, Response},
    server::{AsyncRequest, Server},
};
use ghost_shell_launcher::Launcher;
use ghost_shell_widget_clock::ClockWidget;
use ghost_shell_widget_focus::FocusWidget;
use ghost_shell_widget_menu::MenuWidget;
use ghost_shell_widget_power::PowerWidget;
use ghost_shell_widget_workspaces::WorkspacesWidget;

/// Loads app configuration and opens bars on available displays.
///
/// # Errors
/// Bubbles up errors from bar initialization
///
pub fn init(cx: &mut App) -> Result<()> {
    let config = ghost_shell_config::load()
        .inspect_err(|e| eprintln!("Failed to load config {e:?}"))
        .unwrap_or_default();

    let (request_sender, mut request_receiver) =
        mpsc::channel::<AsyncRequest>(32);
    let ipc_socket_path =
        env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap();
    let ipc_server = Tokio::handle(cx).block_on(Server::bind(
        ipc_socket_path.join("ghost-shell-daemon"),
        request_sender,
    ))?;

    let menu_widget = cx.new(|_cx| MenuWidget {});
    let power_widget = cx.new(|_cx| PowerWidget {});
    let clock_widget = cx.new(|cx| ClockWidget::new(config.clock.clone(), cx));
    let focus_widget = cx.new(|cx| FocusWidget::new(cx));

    for (output_name, bar_config) in config.bars {
        if let Some(display) = cx.displays().into_iter().find(|display| {
            let output_uuid =
                Uuid::new_v5(&Uuid::NAMESPACE_DNS, output_name.as_bytes());
            display.uuid().is_ok_and(|uuid| uuid == output_uuid)
        }) {
            let workspaces_widget =
                cx.new(|cx| WorkspacesWidget::new(cx, output_name));

            ghost_shell_bar::open(
                &display,
                config.general.clone(),
                bar_config,
                menu_widget.clone(),
                workspaces_widget,
                focus_widget.clone(),
                power_widget.clone(),
                clock_widget.clone(),
                cx,
            )?;
        }
    }

    Tokio::spawn(cx, ipc_server.run()).detach();

    let launcher = cx.new(|_| Launcher::new());
    let launcher = launcher.clone();

    let primary_display_name =
        Uuid::new_v5(&Uuid::NAMESPACE_DNS, "DP-1".as_bytes());
    let primary_display = cx
        .displays()
        .into_iter()
        .find(|display| {
            display
                .uuid()
                .is_ok_and(|uuid| uuid == primary_display_name)
        })
        .unwrap();

    // Spawn task to handle request over IPC connection
    cx.spawn(async move |cx| {
        while let Some(incoming) = request_receiver.recv().await {
            let reply: std::result::Result<Response, String> =
                cx.update(|cx| match incoming.request {
                    Request::Launcher { action } => match action {
                        LauncherAction::Toggle => {
                            launcher.update(cx, |launcher, cx| {
                                let _ = launcher.toggle(cx, primary_display.clone()).inspect_err(|err| eprintln!("failed to toggle launcher {err:#}"));
                            });
                            Ok(Response::Handled)
                        }
                    },
                });

            let _ = incoming.reply.send(reply);
        }
    })
    .detach();

    Ok(())
}
