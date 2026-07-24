use gpui::Global;
use serde::Deserialize;

pub struct AppState {
    config: AppConfig,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }
}

impl Global for AppState {}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    bar_height: f32,
    bar_exclusive_zone: f32,
}
