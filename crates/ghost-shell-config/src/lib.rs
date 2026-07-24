use config::{Config, ConfigError, File};
use directories::ProjectDirs;
use ghost_shell_core::AppConfig;

pub fn load() -> Result<AppConfig, ConfigError> {
    let dirs = ProjectDirs::from("dev", "thatwhichis", "ghost-shell")
        .ok_or_else(|| ConfigError::NotFound("app config dir doesn't exist".to_string()))?;

    let config_path = dirs.config_dir().join("config.toml");
    let config = Config::builder()
        .add_source(File::from(config_path.as_path()).required(true))
        .build()?;

    let app_config: AppConfig = config.try_deserialize()?;

    Ok(app_config)
}
