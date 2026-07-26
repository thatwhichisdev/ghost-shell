use std::rc::Rc;

use anyhow::{Context, Result};
use ghost_shell_config::AppConfig;
use gpui::{
    App, IntoElement, PlatformDisplay, Render, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind,
    WindowOptions, div,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    point,
    prelude::*,
    px, rgb, rgba,
};

pub struct WindowBar;

impl Render for WindowBar {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        div()
            .self_flex_end()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb(0x00ff_ffff))
            .bg(rgba(0x0000_0000))
            .text_sm()
            .child("<bar>")
    }
}

/// Open bar for given display, based on display properties will calculate bar width
pub fn open(
    display: Rc<dyn PlatformDisplay>,
    app_config: AppConfig,
    cx: &mut App,
) -> Result<WindowHandle<WindowBar>> {
    let window_options = window_options(display, app_config);

    cx.open_window(window_options, |_, cx| cx.new(|_| WindowBar))
        .context("failed to open bar")
}

fn window_options(
    display: Rc<dyn PlatformDisplay>,
    app_config: AppConfig,
) -> WindowOptions {
    let app_id: String = format!("ghost-shell-{:?}", display.id());
    let namespace: String = format!("namespace-{:?}", display.id());
    let display_size = display.bounds().size;
    let window_size = Size::new(display_size.width, px(app_config.bar_height));

    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(gpui::Bounds {
            origin: point(px(0.0), px(0.0)),
            size: window_size,
        })),
        titlebar: None,
        focus: false,
        show: true,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace,
            layer: Layer::Top,
            anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
            exclusive_zone: Some(px(app_config.bar_exclusive_zone)),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        }),
        is_movable: false,
        app_owns_titlebar_drag: false,
        is_resizable: false,
        is_minimizable: false,
        display_id: Some(display.id()),
        window_background: WindowBackgroundAppearance::Blurred,
        app_id: Some(app_id),
        window_min_size: None,
        window_decorations: None,
        icon: None,
        tabbing_identifier: None,
    }
}
