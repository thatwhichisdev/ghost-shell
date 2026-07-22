use gpui::{
    App, BoxShadow, Context, IntoElement, Render, Size, Window, WindowBackgroundAppearance,
    WindowOptions, div, hsla,
    layer_shell::{Anchor, LayerShellOptions},
    point,
    prelude::*,
    px, rgb, rgba,
};

const BAR_HEIGHT: f32 = 18.0;

struct Pyro;

impl Render for Pyro {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .self_flex_end()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb(0xffffff))
            .bg(rgba(0x00000000))
            .shadow(vec![BoxShadow {
                color: hsla(0.5, 0.5, 0.5, 0.5),
                offset: point(px(0.0), px(0.0)),
                blur_radius: px(12.0),
                spread_radius: px(2.0),
                inset: true,
            }])
            .text_sm()
            .child("pyro freaking fast shell environment built using blazing technlogies such as rust, gpui and wayland yo yo yo this thing goes brrrrrr")
    }
}

#[tokio::main]
async fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let display_size = cx
            .primary_display()
            .map(|display| display.bounds().size)
            .unwrap_or_else(|| Size::new(px(1920.0), px(1080.0)));

        let options = WindowOptions {
            app_id: Some("pyro".to_owned()),
            titlebar: None,
            window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds {
                origin: point(px(0.0), px(0.0)),
                size: Size::new(display_size.width, px(BAR_HEIGHT)),
            })),
            window_background: WindowBackgroundAppearance::Blurred,
            kind: gpui::WindowKind::LayerShell(LayerShellOptions {
                namespace: "pyro-bar".to_string(),
                layer: gpui::layer_shell::Layer::Top,
                anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
                exclusive_zone: Some(px(BAR_HEIGHT)),
                keyboard_interactivity: gpui::layer_shell::KeyboardInteractivity::None,
                ..Default::default()
            }),
            ..Default::default()
        };

        cx.open_window(options, |_, cx| cx.new(|_| Pyro))
            .expect("failed to open the pyro");

        cx.activate(true);
    });
}
