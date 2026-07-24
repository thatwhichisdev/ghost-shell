use gpui::{
    App, Context, IntoElement, Render, Size, Window, WindowBackgroundAppearance, WindowBounds,
    WindowKind, WindowOptions, div,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    point,
    prelude::*,
    px, rgb, rgba,
};

const BAR_HEIGHT: f32 = 18.0;
const EXCLUSIVE_ZONE_HEIGHT: f32 = 9.0;

struct Bar;

impl Render for Bar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .self_flex_end()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb(0xffffff))
            .bg(rgba(0x00000000))
            .text_sm()
            .child("<bar>")
    }
}

fn main() {
    let app = gpui_platform::application();
    let app_config = ghost_shell_config::load().expect("ghost-shell config is not present");
    let app_state = ghost_shell_core::AppState::new(app_config);

    app.run(|cx: &mut App| {
        cx.set_global(app_state);

        for display in cx.displays() {
            let display_size = display.bounds().size;
            let app_id: String = format!("ghost-shell-{:?}", display.id());
            let namespace: String = format!("namespace-{:?}", display.id());

            let window_options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(gpui::Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: Size::new(display_size.width, px(BAR_HEIGHT)),
                })),
                titlebar: None,
                focus: false,
                show: true,
                kind: WindowKind::LayerShell(LayerShellOptions {
                    namespace: namespace,
                    layer: Layer::Top,
                    anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
                    exclusive_zone: Some(px(EXCLUSIVE_ZONE_HEIGHT)),
                    keyboard_interactivity: KeyboardInteractivity::OnDemand,
                    ..Default::default()
                }),
                is_movable: true,
                app_owns_titlebar_drag: false,
                is_resizable: true,
                is_minimizable: true,
                display_id: Some(display.id()),
                window_background: WindowBackgroundAppearance::Blurred,
                app_id: Some(app_id),
                window_min_size: None,
                window_decorations: None,
                icon: None,
                tabbing_identifier: None,
            };

            let _handle = cx
                .open_window(window_options, |_, cx| cx.new(|_| Bar))
                .expect("failed to create window on display");
        }

        cx.activate(true);
    });
}
