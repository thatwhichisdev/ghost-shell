use anyhow::Result;
use gpui::{App, prelude::*};

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

    for display in cx.displays() {
        ghost_shell_bar::open(
            &display,
            config.clone(),
            menu_widget.clone(),
            battery_widget.clone(),
            clock_widget.clone(),
            cx,
        )?;
    }

    Ok(())
}
