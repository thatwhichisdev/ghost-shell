//! Session-lock window lifecycle.
use anyhow::Result;
use gpui::{
    App, AppContext as _, Global, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions,
};
use gpui_component::Root;

use ghost_shell_app::GhostShell;

use crate::view::LockView;

/// Owns the session-lock windows for the active displays.
///
/// A non-empty window set prevents duplicate lock requests.
pub struct LockManager {
    /// Lock screen views for each output
    windows: Vec<WindowHandle<Root>>,
}

impl LockManager {
    /// Creates an unlocked lock manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
        }
    }

    /// Begins session locking and opens a lockscreen window on each active display.
    ///
    /// Returns without changing state if lockscreen windows already exist.
    ///
    /// # Errors
    ///
    /// Returns an error if a session-lock window cannot be opened.
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
                window_background: WindowBackgroundAppearance::Opaque,
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

    /// Unlocks the active session and releases its lockscreen windows.
    ///
    /// Window handles are retained if the session cannot be unlocked.
    ///
    /// # Errors
    ///
    /// Returns an error if GPUI cannot unlock the active Wayland session.
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
