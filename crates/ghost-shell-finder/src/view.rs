use gpui::{Context, IntoElement, Render, Window, div, prelude::*};

pub(crate) struct View;

impl Render for View {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().id("finder").key_context("finder")
    }
}
