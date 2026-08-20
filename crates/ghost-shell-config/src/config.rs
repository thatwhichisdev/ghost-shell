use std::{collections::HashMap, path::PathBuf};

use config::{Config, ConfigError, File};
use directories::ProjectDirs;
use gpui::Global;
use serde::Deserialize;

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(rename = "bar")]
    #[serde(default)]
    pub bars: HashMap<String, BarConfig>,

    #[serde(default)]
    pub clock: ClockConfig,

    #[serde(default)]
    pub wallpaper: WallpaperConfig,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WallpaperConfig {
    pub path: Option<String>,
    pub bg: u32,
}

impl Default for WallpaperConfig {
    fn default() -> Self {
        Self {
            path: None,
            bg: 0x0000_0000,
        }
    }
}

/// Load configuration of the shell by searching config according to XDG base directory specification
///
/// # Errors
/// - `ConfigError::NotFound`
pub fn load() -> Result<AppConfig, ConfigError> {
    let config_dir = config_dir()?;
    let config_path = config_dir.join("config.toml");
    let config = Config::builder()
        .add_source(File::from(config_path.as_path()).required(true))
        .build()?;

    let app_config: AppConfig = config.try_deserialize()?;

    Ok(app_config)
}

/// Get path of the configuration dir
pub fn config_dir() -> Result<PathBuf, ConfigError> {
    project_dir()
        .ok_or_else(|| ConfigError::NotFound("app config dir doesn't exist".to_string()))
        .map(|dirs| dirs.config_dir().to_path_buf())
}

/// Get path of the cache dir
pub fn cache_dir() -> Result<PathBuf, ConfigError> {
    project_dir()
        .ok_or_else(|| ConfigError::NotFound("app cache dir doesn't exist".to_string()))
        .map(|dirs| dirs.cache_dir().to_path_buf())
}

/// Get project directories
pub fn project_dir() -> Option<ProjectDirs> {
    ProjectDirs::from("dev", "thatwhichis", "ghost-shell")
}
