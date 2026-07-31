use gpui::{Context, Window, div, prelude::*, px, rgba, svg};

pub struct PowerWidget;

impl Render for PowerWidget {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().id("power").flex().items_center().child(
            svg()
                .path("battery.svg")
                .size(px(27.0))
                .text_color(rgba(0xffff_ffff)),
        )
    }
}
