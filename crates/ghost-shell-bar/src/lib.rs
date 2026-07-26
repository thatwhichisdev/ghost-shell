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
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x0000_0000))
            .text_color(rgb(0x00ff_ffff))
            .px(px(4.0))
            .text_lg()
            .child(start_section())
            .child(center_section())
            .child(end_section())
    }
}

fn start_section() -> impl IntoElement {
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_start()
        .gap_x(px(2.0))
        .child(mock_widget("menu", "󰍜"))
        .child(mock_widget("workspaces", ""))
}

fn center_section() -> impl IntoElement {
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_x(px(2.0))
        .child(mock_widget("focused", "~/development/mock"))
}

fn end_section() -> impl IntoElement {
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_end()
        .gap_x(px(2.0))
        .child(mock_widget("tray", "󱊔"))
        .child(mock_widget("notifications", ""))
        .child(mock_widget("volume", ""))
        .child(mock_widget("microphone", ""))
        .child(mock_widget("camera", "󰄀"))
        .child(mock_widget("bluetooth", "󰂯"))
        .child(mock_widget("network", "󰤨"))
        .child(mock_widget("battery", "󰁹"))
        .child(mock_widget("clock", "11:56"))
}

pub fn mock_widget(id: &'static str, label: &'static str) -> impl IntoElement {
    div().id(id).flex().items_center().px_2().child(label)
}

/// Open bar for given display, based on display properties will calculate bar width
pub fn open(
    display: Rc<dyn PlatformDisplay>,
    config: AppConfig,
    cx: &mut App,
) -> Result<WindowHandle<WindowBar>> {
    let window_options = window_options(display, config);

    cx.open_window(window_options, |_window, cx| cx.new(|_cx| WindowBar))
        .context("failed to open bar")
}

/// Build `WindowOptions` for given display based on it's properties and application config
fn window_options(
    display: Rc<dyn PlatformDisplay>,
    config: AppConfig,
) -> WindowOptions {
    let app_id: String = "dev.thatwhichis.ghost-shell".to_string();
    let namespace: String = format!("namespace-{:?}", display.id());
    let display_size = display.bounds().size;
    let window_size = Size::new(display_size.width, px(config.bar_height));

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
            exclusive_zone: Some(px(config.bar_exclusive_zone)),
            keyboard_interactivity: KeyboardInteractivity::None,
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
