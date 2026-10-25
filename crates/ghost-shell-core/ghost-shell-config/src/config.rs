use std::{collections::HashMap, path::PathBuf};

use config::{Config, ConfigError, File};
use directories::ProjectDirs;
use serde::Deserialize;

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BarAppConfig {
    pub general: GeneralConfig,
    #[serde(rename = "bar")]
    pub bars: HashMap<String, BarConfig>,
    pub clock: ClockConfig,
    pub wallpaper: WallpaperConfig,
    pub theme: ThemeConfig,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LauncherConfig {
    pub general: GeneralConfig,
    pub launcher: WindowConfig,
    pub theme: ThemeConfig,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FinderConfig {
    pub general: GeneralConfig,
    pub finder: WindowConfig,
    pub theme: ThemeConfig,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LockscreenConfig {
    pub general: GeneralConfig,
    pub wallpaper: WallpaperConfig,
    pub theme: ThemeConfig,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GreeterConfig {
    pub general: GeneralConfig,
    pub wallpaper: WallpaperConfig,
    pub theme: ThemeConfig,
    pub session: GreeterSessionConfig,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GreeterSessionConfig {
    pub command: Vec<String>,
    pub environment: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub font_family: String,
    pub font_size: f32,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            font_family: "monospace".into(),
            font_size: 13.0,
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
    pub appearance: BarAppearance,
    pub background_opacity: Option<f32>,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            output: "<default>".to_string(),
            height: 27.0,
            exclusive_zone: 27.0,
            primary: false,
            appearance: BarAppearance::default(),
            background_opacity: None,
        }
    }
}

impl BarConfig {
    pub fn background_opacity(&self) -> f32 {
        match self.appearance {
            BarAppearance::Themed => background_opacity(false, self.background_opacity),
            BarAppearance::Blur => background_opacity(true, self.background_opacity),
            BarAppearance::Transparent => 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BarAppearance {
    #[default]
    #[serde(alias = "default")]
    Themed,
    Blur,
    Transparent,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    pub blur: bool,
    pub background_opacity: Option<f32>,
}

impl WindowConfig {
    pub fn background_opacity(&self) -> f32 {
        background_opacity(self.blur, self.background_opacity)
    }
}

fn background_opacity(blur: bool, configured_opacity: Option<f32>) -> f32 {
    configured_opacity
        .filter(|opacity| opacity.is_finite())
        .unwrap_or(if blur { 0.8 } else { 1.0 })
        .clamp(0.0, 1.0)
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

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub mode: ThemeMode,
    pub dark: Base16Config,
    pub light: Base16Config,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Dark,
            dark: Base16Config::default_dark(),
            light: Base16Config::default_light(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Base16Config {
    pub base00: u32,
    pub base01: u32,
    pub base02: u32,
    pub base03: u32,
    pub base04: u32,
    pub base05: u32,
    pub base06: u32,
    pub base07: u32,
    pub base08: u32,
    pub base09: u32,

    #[serde(rename = "base0A")]
    pub base0a: u32,

    #[serde(rename = "base0B")]
    pub base0b: u32,

    #[serde(rename = "base0C")]
    pub base0c: u32,

    #[serde(rename = "base0D")]
    pub base0d: u32,

    #[serde(rename = "base0E")]
    pub base0e: u32,

    #[serde(rename = "base0F")]
    pub base0f: u32,
}

impl Base16Config {
    pub fn default_dark() -> Self {
        Self {
            base00: 0x0a0c10,
            base01: 0x272b33,
            base02: 0x7a828e,
            base03: 0x9ea7b3,
            base04: 0xbdc4cc,
            base05: 0xf0f3f6,
            base06: 0xffffff,
            base07: 0xffffff,
            base08: 0xffb757,
            base09: 0x91cbff,
            base0a: 0xe09b13,
            base0b: 0xaddcff,
            base0c: 0x72f088,
            base0d: 0xdbb7ff,
            base0e: 0xff9492,
            base0f: 0xffb1af,
        }
    }

    pub fn default_light() -> Self {
        Self {
            base00: 0xffffff,
            base01: 0xe7ecf0,
            base02: 0xacb6c0,
            base03: 0x88929d,
            base04: 0x66707b,
            base05: 0x343b43,
            base06: 0x20252c,
            base07: 0x0e1116,
            base08: 0x702c00,
            base09: 0x023b95,
            base0a: 0x956400,
            base0b: 0x032563,
            base0c: 0x024c1a,
            base0d: 0x622cbc,
            base0e: 0xa0111f,
            base0f: 0x6e011a,
        }
    }
}

pub fn load<T: serde::de::DeserializeOwned>(name: &str) -> Result<T, ConfigError> {
    let directory = config_dir()?;
    let shared_path = directory.join("config.toml");
    let shared_exists = shared_path.try_exists().map_err(|error| {
        ConfigError::Message(format!(
            "Could not inspect {}: {error}",
            shared_path.display()
        ))
    })?;
    if shared_exists {
        return load_file(&shared_path);
    }
    let config_path = directory.join(format!("{name}.toml"));
    Config::builder()
        .add_source(File::from(config_path.as_path()).required(false))
        .build()?
        .try_deserialize()
}

pub fn load_file<T: serde::de::DeserializeOwned>(
    path: &std::path::Path,
) -> Result<T, ConfigError> {
    Config::builder()
        .add_source(File::from(path).required(true))
        .build()?
        .try_deserialize()
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

#[cfg(test)]
mod tests {
    use config::FileFormat;

    use super::*;

    #[test]
    fn blur_settings_are_independent_and_default_to_disabled() -> Result<(), ConfigError>
    {
        let source = r#"
            [bar.first]
            appearance = "blur"

            [bar.second]
            appearance = "themed"

            [launcher]
            blur = true
            background_opacity = 0.6
        "#;
        let config: BarAppConfig = Config::builder()
            .add_source(File::from_str(source, FileFormat::Toml))
            .build()?
            .try_deserialize()?;

        assert_eq!(
            config
                .bars
                .get("first")
                .map(|bar| bar.appearance),
            Some(BarAppearance::Blur)
        );
        assert_eq!(
            config
                .bars
                .get("second")
                .map(|bar| bar.appearance),
            Some(BarAppearance::Themed)
        );
        assert_eq!(
            config
                .bars
                .get("first")
                .map(BarConfig::background_opacity),
            Some(0.8)
        );
        let launcher: LauncherConfig = Config::builder()
            .add_source(File::from_str(source, FileFormat::Toml))
            .build()?
            .try_deserialize()?;
        assert!(launcher.launcher.blur);
        assert_eq!(launcher.launcher.background_opacity(), 0.6);
        let finder = FinderConfig::default();
        assert!(!finder.finder.blur);
        assert_eq!(finder.finder.background_opacity(), 1.0);
        Ok(())
    }
}
