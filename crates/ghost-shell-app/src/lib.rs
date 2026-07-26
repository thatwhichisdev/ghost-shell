use anyhow::Result;
use ghost_shell_config::AppConfig;
use gpui::App;

pub struct AppState;

impl AppState {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// Initializes shell
///
/// # Errors
/// Bubbles up errors from bar initialization
pub fn init(cx: &mut App) -> Result<()> {
    let app_config = match ghost_shell_config::load() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Failed to load config, using default {err:?}");
            AppConfig::default()
        }
    };

    for display in cx.displays() {
        ghost_shell_bar::open(display, app_config.clone(), cx)?;
    }

    Ok(())
}
