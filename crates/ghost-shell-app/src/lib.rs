use anyhow::Result;
use gpui::App;

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

    for display in cx.displays() {
        ghost_shell_bar::open(display, config.clone(), cx)?;
    }

    Ok(())
}
