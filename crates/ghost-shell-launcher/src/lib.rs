use std::rc::Rc;

use anyhow::{Context as _, Result};
use gpui::{
    AnyWindowHandle, Bounds, Context, IntoElement, PlatformDisplay, Render,
    Window, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, div,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    prelude::*,
    px, rgb, size,
};
use gpui_component::Root;

pub struct Launcher {
    window: Option<AnyWindowHandle>,
}

impl Launcher {
    #[must_use]
    pub fn new() -> Self {
        Self { window: None }
    }

    pub fn toggle(
        &mut self,
        cx: &mut Context<Self>,
        display: Rc<dyn PlatformDisplay>,
    ) -> Result<()> {
        match self.window.is_some() {
            true => self.close(cx),
            false => self.open(cx, display),
        }
    }

    pub fn open(
        &mut self,
        cx: &mut Context<Self>,
        display: Rc<dyn PlatformDisplay>,
    ) -> Result<()> {
        let bounds = Bounds::centered(
            Some(display.id()),
            size(px(400.0), px(400.0)),
            cx,
        );

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            focus: true,
            show: true,
            kind: WindowKind::LayerShell(LayerShellOptions {
                namespace: "ghost-shell-launcher".to_owned(),
                layer: Layer::Overlay,
                anchor: Anchor::empty(),
                exclusive_zone: None,
                exclusive_edge: None,
                margin: None,
                keyboard_interactivity: KeyboardInteractivity::OnDemand,
            }),
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            display_id: Some(display.id()),
            window_background: WindowBackgroundAppearance::Transparent,
            app_id: Some("ghost-shell-launcher".to_owned()),
            ..Default::default()
        };

        let handle = cx.open_window(window_options, |window, cx| {
            let view = cx.new(|_| LauncherView);
            cx.new(|cx| Root::new(view, window, cx))
        })?;

        self.window = Some(handle.into());

        Ok(())
    }

    pub fn close(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let Some(handle) = self.window.take() else {
            return Ok(());
        };

        handle
            .update(cx, |_view, window, _cx| {
                window.remove_window();
            })
            .context("failed to close launcher window")?;

        Ok(())
    }
}

struct LauncherView;

impl Render for LauncherView {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id("launcher")
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .rounded_lg()
            .border_1()
            .border_color(rgb(0x444444))
            .bg(rgb(0x181818))
            .text_color(rgb(0xffffff))
            .child("Ghost Launcher")
    }
}
