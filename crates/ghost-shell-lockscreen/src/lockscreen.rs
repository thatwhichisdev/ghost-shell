use anyhow::Result;
use gpui::{
    App, AppContext as _, Global, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions,
};
use gpui_component::Root;

use ghost_shell_app::GhostShell;

use crate::view::LockView;

/// Struct to represent lockscreen and it's state
pub struct LockManager {
    /// Lock screen views for each output
    windows: Vec<WindowHandle<Root>>,
}

impl LockManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
        }
    }

    pub fn lock(&mut self, cx: &mut App) -> Result<()> {
        if !self.windows.is_empty() {
            return Ok(());
        }

        for display in cx.global::<GhostShell>().get_displays() {
            let window_options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(display.bounds())),
                titlebar: None,
                kind: WindowKind::SessionLock,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                display_id: Some(display.id()),
                window_background: WindowBackgroundAppearance::Transparent,
                app_id: Some("ghost-shell-lockscreen".to_owned()),
                ..Default::default()
            };

            let handle = cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| LockView::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx).bordered(false))
            })?;

            self.windows.push(handle);
        }

        Ok(())
    }

    pub(crate) fn unlock(&mut self, cx: &mut App) -> Result<()> {
        cx.unlock_session()?;

        self.windows.clear();

        Ok(())
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Global for LockManager {}
