use anyhow::Result;
use ghost_shell_niri::client::client::NiriClient;
use gpui::{App, accesskit::Uuid, prelude::*};
use gpui_tokio::Tokio;

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

    let menu_widget = cx.new(|_cx| ghost_shell_system::menu::Menu {});
    let battery_widget = cx.new(|_cx| ghost_shell_power::battery::Battery {});
    let clock_widget = cx.new(|cx| {
        ghost_shell_system::clock::Clock::new(config.clock.clone(), cx)
    });

    let niri_client = Tokio::handle(cx).block_on(NiriClient::new()).unwrap();

    let niri_event_reader =
        Tokio::handle(cx).block_on(niri_client.into_event_reader());

    let niri_event_receiver = niri_event_reader.subscribe();

    Tokio::spawn(cx, niri_event_reader.run()).detach();

    let focus_widget = cx
        .new(|cx| ghost_shell_niri::focus::Focus::new(cx, niri_event_receiver));

    for (output_name, bar_config) in config.bars {
        if let Some(display) = cx.displays().into_iter().find(|display| {
            let output_uuid =
                Uuid::new_v5(&Uuid::NAMESPACE_DNS, output_name.as_bytes());
            display.uuid().is_ok_and(|uuid| uuid == output_uuid)
        }) {
            ghost_shell_bar::open(
                &display,
                config.general.clone(),
                bar_config,
                menu_widget.clone(),
                focus_widget.clone(),
                battery_widget.clone(),
                clock_widget.clone(),
                cx,
            )?;
        }
    }

    Ok(())
}
