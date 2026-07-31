use gpui::{Context, Window, div, prelude::*, px, rgba, svg};
use gpui_component::{Icon, Sizable};

pub struct Menu;

impl Render for Menu {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id("menu")
            .flex()
            .items_center()
            .pl(px(8.0))
            .child(Icon::empty().path("nixos.svg").with_size(px(18.0)))
    }
}
