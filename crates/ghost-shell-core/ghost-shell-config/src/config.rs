use std::{collections::HashMap, path::PathBuf};

use config::{Config, ConfigError, File};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BarAppConfig {
    pub general: GeneralConfig,
    pub bar: BarConfig,
    pub clock: ClockConfig,
    pub wallpaper: WallpaperConfig,
    pub theme: ThemeConfig,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherConfig {
    pub general: GeneralConfig,
    pub launcher: WindowConfig,
    pub theme: ThemeConfig,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FinderConfig {
    pub general: GeneralConfig,
    pub finder: WindowConfig,
    pub theme: ThemeConfig,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LockscreenConfig {
    pub general: GeneralConfig,
    pub wallpaper: WallpaperConfig,
    pub theme: ThemeConfig,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GreeterConfig {
    pub general: GeneralConfig,
    pub wallpaper: WallpaperConfig,
    pub theme: ThemeConfig,
    pub session: GreeterSessionConfig,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GreeterSessionConfig {
    pub command: Vec<String>,
    pub environment: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub font_family: String,
    #[serde(deserialize_with = "positive_dimension")]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BarConfig {
    #[serde(deserialize_with = "positive_dimension")]
    pub height: f32,
    #[serde(deserialize_with = "nonnegative_dimension")]
    pub exclusive_zone: f32,
    pub outputs: HashMap<String, BarOutputConfig>,
    pub decoration: BarDecorationConfig,
    pub widgets: BarWidgetsConfig,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            height: 28.0,
            exclusive_zone: 28.0,
            outputs: HashMap::new(),
            decoration: BarDecorationConfig::default(),
            widgets: BarWidgetsConfig::default(),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BarOutputConfig {
    pub primary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BarDecorationConfig {
    pub layout: BarLayout,
    #[serde(deserialize_with = "opacity")]
    pub opacity: f32,
    pub blur: bool,
    #[serde(deserialize_with = "nonnegative_dimension")]
    pub radius: f32,
}

impl Default for BarDecorationConfig {
    fn default() -> Self {
        Self {
            layout: BarLayout::Continuous,
            opacity: 1.0,
            blur: false,
            radius: 4.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BarLayout {
    #[default]
    Continuous,
    Islands,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BarWidgetsConfig {
    #[serde(deserialize_with = "positive_dimension")]
    pub height: f32,
    #[serde(deserialize_with = "positive_dimension")]
    pub icon_size: f32,
    #[serde(deserialize_with = "nonnegative_dimension")]
    pub padding_horizontal: f32,
    #[serde(deserialize_with = "nonnegative_dimension")]
    pub margin_horizontal: f32,
    #[serde(deserialize_with = "nonnegative_dimension")]
    pub margin_vertical: f32,
}

impl Default for BarWidgetsConfig {
    fn default() -> Self {
        Self {
            height: 20.0,
            icon_size: 16.0,
            padding_horizontal: 4.0,
            margin_horizontal: 2.0,
            margin_vertical: 4.0,
        }
    }
}

fn positive_dimension<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<f32, D::Error> {
    let value = nonnegative_dimension(deserializer)?;
    if value > 0.0 {
        Ok(value)
    } else {
        Err(serde::de::Error::custom("dimension must be positive"))
    }
}

fn nonnegative_dimension<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<f32, D::Error> {
    let value = f32::deserialize(deserializer)?;
    if value.is_finite() && (0.0..=16384.0).contains(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(
            "dimension must be finite and between 0 and 16384",
        ))
    }
}

fn opacity<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<f32, D::Error> {
    let value = f32::deserialize(deserializer)?;
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom("opacity must be between 0 and 1"))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub icons: Option<String>,
    pub mode: ThemeMode,
    pub dark: Base16Config,
    pub light: Base16Config,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            icons: None,
            mode: ThemeMode::Dark,
            dark: Base16Config::default_dark(),
            light: Base16Config::default_light(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Base16Config {
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base00: u32,
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base01: u32,
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base02: u32,
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base03: u32,
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base04: u32,
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base05: u32,
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base06: u32,
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base07: u32,
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base08: u32,
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base09: u32,

    #[serde(rename = "base0A")]
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base0a: u32,

    #[serde(rename = "base0B")]
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base0b: u32,

    #[serde(rename = "base0C")]
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base0c: u32,

    #[serde(rename = "base0D")]
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base0d: u32,

    #[serde(rename = "base0E")]
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base0e: u32,

    #[serde(rename = "base0F")]
    #[serde(deserialize_with = "deserialize_theme_color")]
    pub base0f: u32,
}

fn deserialize_theme_color<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct ThemeColorVisitor;

    impl<'de> serde::de::Visitor<'de> for ThemeColorVisitor {
        type Value = u32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a #RRGGBB color string or an unsigned 32-bit integer")
        }

        fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<u32, E> {
            let digits = value.strip_prefix('#').filter(|digits| {
                digits.len() == 6
                    && digits
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit())
            });
            match digits {
                Some(digits) => u32::from_str_radix(digits, 16).map_err(E::custom),
                None => Err(E::custom(
                    "theme colors must use #RRGGBB with exactly six hexadecimal digits",
                )),
            }
        }

        fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<u32, E> {
            u32::try_from(value).map_err(E::custom)
        }

        fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<u32, E> {
            u32::try_from(value).map_err(E::custom)
        }
    }

    deserializer.deserialize_any(ThemeColorVisitor)
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
    load_directory(&config_dir()?, name)
}

pub(crate) fn load_directory<T: serde::de::DeserializeOwned>(
    directory: &std::path::Path,
    name: &str,
) -> Result<T, ConfigError> {
    let shared_path = directory.join("config.toml");
    let shared_exists = match shared_path.symlink_metadata() {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
    .map_err(|error| {
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

    fn parse_theme_color(value: &str) -> Result<ThemeConfig, ConfigError> {
        use std::fmt::Write;

        let mut source = String::new();
        for polarity in ["dark", "light"] {
            writeln!(source, "[theme.{polarity}]")
                .map_err(|error| ConfigError::Message(error.to_string()))?;
            for index in 0..16 {
                writeln!(source, "base{index:02X} = {value}")
                    .map_err(|error| ConfigError::Message(error.to_string()))?;
            }
        }
        let config: BarAppConfig = Config::builder()
            .add_source(File::from_str(&source, FileFormat::Toml))
            .build()?
            .try_deserialize()?;
        Ok(config.theme)
    }

    #[test]
    fn theme_palettes_accept_hex_strings_and_legacy_numbers() -> Result<(), ConfigError> {
        for (value, expected) in [
            ("\"#0a0C10\"", 0x0a0c10),
            ("\"#000000\"", 0),
            ("\"#FFFFFF\"", 0xffffff),
            ("658448", 0x0a0c10),
            ("0x0a0c10", 0x0a0c10),
        ] {
            let theme = parse_theme_color(value)?;
            for palette in [theme.dark, theme.light] {
                let colors = [
                    palette.base00,
                    palette.base01,
                    palette.base02,
                    palette.base03,
                    palette.base04,
                    palette.base05,
                    palette.base06,
                    palette.base07,
                    palette.base08,
                    palette.base09,
                    palette.base0a,
                    palette.base0b,
                    palette.base0c,
                    palette.base0d,
                    palette.base0e,
                    palette.base0f,
                ];
                assert_eq!(colors, [expected; 16], "input: {value}");
            }
        }
        Ok(())
    }

    #[test]
    fn theme_palettes_reject_invalid_colors() {
        for value in [
            "\"#abc\"",
            "\"#12345678\"",
            "\"#gg0000\"",
            "\"123456\"",
            "\"0x123456\"",
            "\"#１２３４５６\"",
            "\" #123456\"",
            "\"#123456 \"",
            "-1",
            "4294967296",
            "1.5",
            "true",
            "[]",
        ] {
            assert!(parse_theme_color(value).is_err(), "accepted {value}");
        }
    }

    #[test]
    fn global_bar_settings_and_icon_theme() -> Result<(), ConfigError> {
        let source = r#"
            [bar]
            height = 32
            exclusive_zone = 32
            [bar.outputs.first]
            primary = true
            [bar.outputs.second]
            [bar.decoration]
            layout = "islands"
            opacity = 0.6
            blur = true
            [bar.widgets]
            icon_size = 18
            [theme]
            icons = "Papirus-Dark"
            [launcher]
            blur = true
            background_opacity = 0.6
        "#;
        let config: BarAppConfig = Config::builder()
            .add_source(File::from_str(source, FileFormat::Toml))
            .build()?
            .try_deserialize()?;
        assert_eq!(config.bar.outputs.len(), 2);
        assert!(
            config
                .bar
                .outputs
                .get("first")
                .is_some_and(|output| output.primary)
        );
        assert_eq!(config.bar.height, 32.0);
        assert_eq!(config.bar.decoration.layout, BarLayout::Islands);
        assert!(config.bar.decoration.blur);
        assert_eq!(config.bar.decoration.opacity, 0.6);
        assert_eq!(config.bar.widgets.icon_size, 18.0);
        assert_eq!(config.bar.widgets.height, 20.0);
        let launcher: LauncherConfig = Config::builder()
            .add_source(File::from_str(source, FileFormat::Toml))
            .build()?
            .try_deserialize()?;
        assert_eq!(config.theme.icons, launcher.theme.icons);
        assert_eq!(launcher.theme.icons.as_deref(), Some("Papirus-Dark"));
        assert!(launcher.launcher.blur);
        assert!(!BarConfig::default().decoration.blur);
        Ok(())
    }

    #[test]
    fn invalid_bar_values_are_rejected() {
        for source in [
            "[bar]\nheight = -1",
            "[bar.widgets]\nicon_size = 0",
            "[bar.decoration]\nopacity = 1.5",
            "[bar.decoration]\nradius = nan",
            "[bar.first]\nappearance = \"blur\"",
        ] {
            let parsed = Config::builder()
                .add_source(File::from_str(source, FileFormat::Toml))
                .build()
                .and_then(|config| config.try_deserialize::<BarAppConfig>());
            assert!(parsed.is_err(), "accepted {source}");
        }
    }
}
