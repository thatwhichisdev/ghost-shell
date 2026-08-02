pub mod config;

pub use config::*;

use gpui::App;

pub fn init(cx: &mut App) {
    let config = config::load()
        .inspect_err(|e| eprintln!("Failed to load config {e:?}"))
        .unwrap_or_default();

    cx.set_global(config);
}
