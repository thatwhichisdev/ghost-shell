use anyhow::Result;
use ghost_shell_app::GhostShell;
use gpui::{
    AnyWindowHandle, App, AppContext as _, Global, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions,
};
use gpui_component::Root;

use crate::view::View;

pub struct Lockscreen {
    windows: Vec<AnyWindowHandle>,
}

impl Lockscreen {
    #[must_use]
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
        }
    }

    pub fn open(&mut self, cx: &mut App) -> Result<()> {
        // Ignore duplicate lock requests while our lockscreen windows exist.
        if !self.windows.is_empty() {
            return Ok(());
        }

        let displays = cx
            .global::<GhostShell>()
            .outputs
            .iter()
            .map(|output| output.display.clone())
            .collect::<Vec<_>>();

        anyhow::ensure!(
            !displays.is_empty(),
            "cannot lock session without any available outputs"
        );

        for display in displays {
            let window_options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(display.bounds())),
                titlebar: None,
                focus: true,
                show: true,
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
                let view = cx.new(|cx| View::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx).bordered(false))
            })?;

            self.windows.push(handle.into());
        }

        Ok(())
    }

    pub(crate) fn clear(&mut self) {
        self.windows.clear();
    }
}

impl Global for Lockscreen {}

impl Default for Lockscreen {
    fn default() -> Self {
        Self::new()
    }
}
