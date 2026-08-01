use std::rc::Rc;

use ghost_shell_config::BarConfig;
use ghost_shell_widget_clock::ClockWidget;
use ghost_shell_widget_focus::FocusWidget;
use ghost_shell_widget_menu::MenuWidget;
use ghost_shell_widget_power::PowerWidget;
use ghost_shell_widget_workspaces::WorkspacesWidget;
use gpui::{
    AnyWindowHandle, App, Entity, IntoElement, PlatformDisplay, Render, Size,
    Window, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, div,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    point,
    prelude::*,
    px,
};
use gpui_component::{Root, ThemeMode};

pub struct Bar {
    config: BarConfig,
    view: Entity<BarView>,
    display: Rc<dyn PlatformDisplay>,
    window: Option<AnyWindowHandle>,
}

pub struct BarView {
    widgets: Widgets,
}

pub struct Widgets {
    pub menu: Entity<MenuWidget>,
    pub workspaces: Entity<WorkspacesWidget>,
    pub focus: Entity<FocusWidget>,
    pub power: Entity<PowerWidget>,
    pub clock: Entity<ClockWidget>,
}

impl Bar {
    #[must_use]
    pub fn new(
        cx: &mut App,
        config: BarConfig,
        widgets: Widgets,
        display: Rc<dyn PlatformDisplay>,
    ) -> Entity<Self> {
        let view = cx.new(|_| BarView { widgets });

        cx.new(|_| Self {
            config,
            view,
            display,
            window: None,
        })
    }

    /// Opens bar and draws it's view
    ///
    /// # Panics
    /// Panics with fails to open the bar.
    ///
    pub fn open(&mut self, cx: &mut App) {
        let window_options = {
            let app_id: String = "dev.thatwhichis.ghost-shell".to_string();
            let namespace: String = "ghost-shell-bar".to_string();

            let size = self.display.bounds().size;
            let size = Size::new(size.width, px(self.config.height));

            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(gpui::Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size,
                })),
                titlebar: None,
                focus: false,
                show: true,
                kind: WindowKind::LayerShell(LayerShellOptions {
                    namespace,
                    layer: Layer::Top,
                    anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
                    exclusive_zone: Some(px(self.config.exclusive_zone)),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    ..Default::default()
                }),
                is_movable: false,
                app_owns_titlebar_drag: false,
                is_resizable: false,
                is_minimizable: false,
                display_id: Some(self.display.id()),
                window_background: WindowBackgroundAppearance::Blurred,
                app_id: Some(app_id),
                window_min_size: None,
                window_decorations: None,
                icon: None,
                tabbing_identifier: None,
            }
        };

        let handle = cx
            .open_window(window_options, |window, cx| {
                gpui_component::theme::Theme::change(
                    ThemeMode::Dark,
                    Some(window),
                    cx,
                );

                cx.new(|cx| {
                    Root::new(self.view.clone(), window, cx).bordered(false)
                })
            })
            .unwrap();

        self.window = Some(handle.into());
    }
}

impl Render for BarView {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        div()
            // .font_family(self.config.font_family.clone())
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            // .bg(rgba(self.config.bg))
            // .text_color(rgba(self.config.fg))
            .px(px(4.0))
            // .text_size(px(self.config.font_size))
            .child({
                div()
                    .flex()
                    .flex_1()
                    .gap_x_2()
                    .items_center()
                    .justify_start()
                    .child(self.widgets.menu.clone())
                    .child(self.widgets.workspaces.clone())
            })
            .child({
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .child(self.widgets.focus.clone())
            })
            .child({
                div()
                    .flex()
                    .flex_1()
                    .gap_x_2()
                    .items_center()
                    .justify_end()
                    .child(self.widgets.power.clone())
                    .child(self.widgets.clock.clone())
            })
    }
}
