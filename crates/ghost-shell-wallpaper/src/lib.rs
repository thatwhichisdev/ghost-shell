pub mod wallpaper;

use gpui::App;

use crate::wallpaper::WallpaperManager;

pub fn init(cx: &mut App) {
    let manager = WallpaperManager::new(cx);
    cx.set_global(manager);
}
