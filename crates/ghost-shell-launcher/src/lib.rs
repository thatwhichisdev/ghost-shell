mod actions;
mod entries;
mod launcher;
mod view;

use gpui::App;

/// Entry point for launcher initialization.
/// Responsible for dekstop entries discovery and basic launcher initialization.
pub fn init(cx: &mut App) {
    entries::init(cx);
    launcher::init(cx);
}
