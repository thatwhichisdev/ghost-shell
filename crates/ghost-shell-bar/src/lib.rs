use gpui::{Context, IntoElement, Render, Window, div, prelude::*, rgb, rgba};

pub struct Bar;

impl Render for Bar {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
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
