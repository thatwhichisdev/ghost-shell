use gpui::{Context, Window, div, prelude::*, px};
use gpui_component::{ActiveTheme as _, Icon, Sizable};

pub struct MenuWidget;

impl Render for MenuWidget {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().id("menu").flex().items_center().pl(px(8.0)).child(
            Icon::empty()
                .path("icons/nixos.svg")
                .with_size(px(18.0))
                .text_color(cx.theme().colors.foreground),
        )
    }
}
