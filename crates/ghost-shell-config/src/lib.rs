use config::{Config, ConfigError, File};
use directories::ProjectDirs;
use gpui::Global;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub bar_height: f32,
    pub bar_exclusive_zone: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            bar_height: 27.0,
            bar_exclusive_zone: 9.0,
        }
    }
}

impl Global for AppConfig {}

/// Load configuration of the shell by searching config according to XDG base directory specification
///
/// # Errors
/// - `ConfigError::NotFound` when directory or file is not found
/// - `ConfigError::FileParse` when file is in wrong format or contains errors
pub fn load() -> Result<AppConfig, ConfigError> {
    let dirs = ProjectDirs::from("dev", "thatwhichis", "ghost-shell")
        .ok_or_else(|| {
            ConfigError::NotFound("app config dir doesn't exist".to_string())
        })?;

    let config_path = dirs.config_dir().join("config.toml");
    let config = Config::builder()
        .add_source(File::from(config_path.as_path()).required(true))
        .build()?;

    let app_config: AppConfig = config.try_deserialize()?;

    Ok(app_config)
}
