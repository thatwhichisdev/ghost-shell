use ghost_shell_gpui::{
    AppContext, Context, IntoElement, Render, Window, WindowOptions, div, prelude::*,
};

struct HelloShell;

impl Render for HelloShell {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(ghost_shell_gpui::rgb(0x181818))
            .text_color(ghost_shell_gpui::rgb(0xeeeeee))
            .child("Ghost GPUI — Wayland")
    }
}

fn main() {
    ghost_shell_gpui::application().run(|cx| {
        if let Err(error) =
            cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| HelloShell))
        {
            eprintln!("Failed to open the GPUI example: {error:#}");
            cx.quit();
        }
    });
}
