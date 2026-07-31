use std::rc::Rc;

use anyhow::{Context, Result};
use ghost_shell_config::{BarConfig, GeneralConfig};
use ghost_shell_niri::{focus::Focus, workspaces::Workspaces};
use ghost_shell_power::battery::Battery;
use ghost_shell_system::{clock::Clock, menu::Menu};
use gpui::{
    App, Entity, IntoElement, PlatformDisplay, Render, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind,
    WindowOptions, div,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    point,
    prelude::*,
    px, rgba,
};
use gpui_component::{Root, ThemeMode};

pub struct Bar {
    config: GeneralConfig,
    menu_widget: Entity<Menu>,
    workspaces_widget: Entity<Workspaces>,
    focus_widget: Entity<Focus>,
    battery_widget: Entity<Battery>,
    clock_widget: Entity<Clock>,
}

impl Render for Bar {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        div()
            .font_family(self.config.font_family.clone())
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(self.config.bg))
            .text_color(rgba(self.config.fg))
            .px(px(4.0))
            .text_size(px(self.config.font_size))
            .child(start_section(
                self.menu_widget.clone(),
                self.workspaces_widget.clone(),
            ))
            .child(center_section(self.focus_widget.clone()))
            .child(end_section(
                self.battery_widget.clone(),
                self.clock_widget.clone(),
            ))
    }
}

fn start_section(
    menu_widget: Entity<Menu>,
    workspaces_widget: Entity<Workspaces>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_1()
        .gap_x_2()
        .items_center()
        .justify_start()
        .child(menu_widget)
        .child(workspaces_widget)
}

fn center_section(focus_widget: Entity<Focus>) -> impl IntoElement {
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_center()
        .child(focus_widget)
}

fn end_section(
    battery_widget: Entity<Battery>,
    clock_widget: Entity<Clock>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_1()
        .gap_x_2()
        .items_center()
        .justify_end()
        .child(battery_widget)
        .child(clock_widget)
}

/// Open bar for given display, based on display properties will calculate bar width
pub fn open(
    display: &Rc<dyn PlatformDisplay>,
    general_config: GeneralConfig,
    bar_config: BarConfig,
    menu_widget: Entity<Menu>,
    workspaces_widget: Entity<Workspaces>,
    focus_widget: Entity<Focus>,
    battery_widget: Entity<Battery>,
    clock_widget: Entity<Clock>,
    cx: &mut App,
) -> Result<WindowHandle<Root>> {
    let window_options = window_options(display, &bar_config);

    cx.open_window(window_options, |window, cx| {
        gpui_component::theme::Theme::change(ThemeMode::Dark, Some(window), cx);

        let bar = Bar {
            config: general_config,
            menu_widget,
            workspaces_widget,
            focus_widget,
            battery_widget,
            clock_widget,
        };
        let view = cx.new(|_| bar);
        cx.new(|cx| Root::new(view, window, cx).bordered(false))
    })
    .context("failed to open bar")
}

/// Build `WindowOptions` for given display based on it's properties and application config
fn window_options(
    display: &Rc<dyn PlatformDisplay>,
    config: &BarConfig,
) -> WindowOptions {
    let app_id: String = "dev.thatwhichis.ghost-shell".to_string();
    let namespace: String = "ghost-shell-bar".to_string();
    let display_size = display.bounds().size;
    let window_size = Size::new(display_size.width, px(config.height));

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
            exclusive_zone: Some(px(config.exclusive_zone)),
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
