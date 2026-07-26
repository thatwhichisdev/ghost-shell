use std::rc::Rc;

use anyhow::{Context, Result};
use ghost_shell_config::AppConfig;
use ghost_shell_system::clock::Clock;
use gpui::{
    App, Entity, IntoElement, PlatformDisplay, Render, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind,
    WindowOptions, div,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    point,
    prelude::*,
    px, rgb, rgba,
};

pub struct Bar {
    clock_widget: Entity<Clock>,
}

impl Render for Bar {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        div()
            .font_family("BerkeleyMono Nerd Font Mono")
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x0000_0000))
            .text_color(rgb(0x00ff_ffff))
            .px(px(4.0))
            .text_sm()
            .child(start_section())
            .child(center_section())
            .child(end_section(self.clock_widget.clone()))
    }
}

fn start_section() -> impl IntoElement {
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_start()
        .child(mock_widget("menu", "󰍜"))
        .child(mock_widget("workspaces", ""))
}

fn center_section() -> impl IntoElement {
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_center()
        .child(mock_widget("focused", "~/development/mock"))
}

fn end_section(
    clock_widget: Entity<ghost_shell_system::clock::Clock>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_end()
        .child(mock_widget("tray", "󱊔"))
        .child(mock_widget("notifications", ""))
        .child(mock_widget("volume", ""))
        .child(mock_widget("microphone", ""))
        .child(mock_widget("camera", "󰄀"))
        .child(mock_widget("bluetooth", "󰂯"))
        .child(mock_widget("network", "󰤨"))
        .child(mock_widget("battery", "󰁹"))
        .child(clock_widget)
}

pub fn mock_widget(id: &'static str, label: &'static str) -> impl IntoElement {
    div().id(id).flex().items_center().px_2().child(label)
}

/// Open bar for given display, based on display properties will calculate bar width
pub fn open(
    display: Rc<dyn PlatformDisplay>,
    config: AppConfig,
    clock_widget: Entity<ghost_shell_system::clock::Clock>,
    cx: &mut App,
) -> Result<WindowHandle<Bar>> {
    let window_options = window_options(display, config);
    let bar = Bar { clock_widget };

    cx.open_window(window_options, |_window, cx| cx.new(|_cx| bar))
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
