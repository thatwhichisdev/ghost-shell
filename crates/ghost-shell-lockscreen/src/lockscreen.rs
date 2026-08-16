use anyhow::Result;
use ghost_shell_app::GhostShell;
use gpui::{
    AnyWindowHandle, App, AppContext as _, BorrowAppContext as _, Context,
    FocusHandle, Focusable, Global, IntoElement, Render, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div,
    prelude::*, px, rgb, rgba,
};

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

        // Clone the displays before opening windows so we don't hold a borrow
        // of GhostShell while mutating GPUI's window state.
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
                let view = cx.new(LockscreenView::new);

                let focus_handle = view.read(cx).focus_handle.clone();
                window.focus(&focus_handle, cx);

                view
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

pub struct LockscreenView {
    focus_handle: FocusHandle,
}

impl LockscreenView {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Focusable for LockscreenView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for LockscreenView {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id("lockscreen")
            .key_context("lockscreen")
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .bg(rgba(0x00000000))
            .text_color(rgb(0xf5f5f5))
            .child(div().text_size(px(28.0)).child("Ghost Shell"))
            .child(div().text_size(px(14.0)).child("Press Enter to unlock"))
    }
}
