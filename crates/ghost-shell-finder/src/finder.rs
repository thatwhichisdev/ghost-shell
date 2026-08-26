use anyhow::{Context as _, Result};
use ghost_shell_actions::{FinderClose, FinderToggle};
use ghost_shell_app::GhostShell;
use gpui::{
    App, AppContext, BorrowAppContext as _, Bounds, Global, KeyBinding,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
    actions, px, size,
};
use gpui_component::Root;

use crate::view::FinderView;

mod search;
mod view;

actions!(finder, [FinderOpenSelected, FinderPreviewSelected]);

/// Initializes the finder and registers its actions and key bindings.
///
/// The finder is stored as a GPUI global and starts without an open window.
pub fn init(cx: &mut App) {
    cx.set_global(Finder::new());

    cx.bind_keys([
        KeyBinding::new("escape", FinderClose, Some("finder")),
        KeyBinding::new("ctrl-o", FinderOpenSelected, Some("finder")),
        KeyBinding::new("ctrl-p", FinderPreviewSelected, Some("finder")),
    ]);

    cx.on_action(|_: &FinderClose, cx| {
        cx.defer(|cx| {
            cx.update_global::<Finder, _>(|finder, cx| match finder.close(cx) {
                Ok(()) => {}
                Err(err) => log::error!("Failed to close launcher {err:#}"),
            });
        });
    });

    cx.on_action(|_: &FinderToggle, cx| {
        cx.update_global::<Finder, _>(|finder, cx| match finder.toggle(cx) {
            Ok(()) => {}
            Err(err) => log::error!("Failed to toggle launcher {err:#}"),
        });
    });
}

/// Manages the lifecycle of the finder window.
///
/// `Finder` keeps a handle to the currently open window. A `None` handle means
/// that no finder window is currently managed.
pub struct Finder {
    handle: Option<WindowHandle<Root>>,
}

impl Finder {
    /// Creates a finder without an open window.
    #[must_use]
    pub fn new() -> Self {
        Self { handle: None }
    }

    /// Toggles the finder window.
    ///
    /// Opens the finder when it is closed and closes it when it is open.
    ///
    /// # Errors
    ///
    /// Returns an error if opening or closing the finder window fails.
    pub fn toggle(&mut self, cx: &mut App) -> Result<()> {
        if self.handle.is_some() {
            self.close(cx)
        } else {
            self.open(cx)
        }
    }

    /// Opens the finder window on the current Ghost Shell output.
    ///
    /// The window is centered on the output and created as a transparent,
    /// non-resizable normal window.
    ///
    /// # Errors
    ///
    /// Returns an error if GPUI fails to create the window.
    pub fn open(&mut self, cx: &mut App) -> Result<()> {
        let output = cx.global::<GhostShell>().get_output();

        let window_size = size(px(900.0), px(700.0));
        let window_bounds = Bounds::centered(Some(output.display.id()), window_size, cx);
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: None,
            kind: WindowKind::Normal,
            is_resizable: false,
            is_minimizable: false,
            display_id: Some(output.display.id()),
            window_background: WindowBackgroundAppearance::Transparent,
            app_id: Some("ghost-shell-finder".to_owned()),
            ..Default::default()
        };

        let handle = cx.open_window(window_options, |window, cx| {
            let view = cx.new(|cx| FinderView::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx).bordered(false))
        })?;

        self.handle = Some(handle);

        Ok(())
    }

    /// Closes the finder window if one is currently open.
    ///
    /// Calling this method while the finder is already closed is a no-op.
    ///
    /// # Errors
    ///
    /// Returns an error if the stored window handle no longer refers to a
    /// window that can be updated.
    pub fn close(&mut self, cx: &mut App) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle
                .update(cx, |_view, window, _cx| window.remove_window())
                .context("failed to close finder window")?;
        }

        Ok(())
    }
}

impl Global for Finder {}
