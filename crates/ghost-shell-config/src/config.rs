use std::collections::HashMap;

use config::{Config, ConfigError, File};
use directories::ProjectDirs;
use gpui::Global;
use serde::Deserialize;

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub general: GeneralConfig,

    #[serde(rename = "bar")]
    pub bars: HashMap<String, BarConfig>,

    pub clock: ClockConfig,
}

impl Global for AppConfig {}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub font_family: String,
    pub font_size: f32,
    pub fg: u32,
    pub bg: u32,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            font_family: "monospace".into(),
            font_size: 13.0,
            fg: 0xffff_ffff,
            bg: 0x0000_0000,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BarConfig {
    pub output: String,
    pub height: f32,
    pub exclusive_zone: f32,
    pub primary: bool,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            output: "<default>".to_string(),
            height: 27.0,
            exclusive_zone: 27.0,
            primary: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ClockConfig {
    pub format: String,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            format: "%H:%M".into(),
        }
    }
}

/// Load configuration of the shell by searching config according to XDG base directory specification
///
/// # Errors
/// - `ConfigError::NotFound`
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
