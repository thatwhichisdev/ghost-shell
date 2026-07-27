use gpui::{Context, Window, div, prelude::*, px, rgba, svg};

pub struct Menu;

impl Render for Menu {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().id("menu").flex().items_center().pl(px(8.0)).child(
            svg()
                .path("nixos.svg")
                .size(px(18.0))
                .text_color(rgba(0xffff_ffff)),
        )
    }
}
