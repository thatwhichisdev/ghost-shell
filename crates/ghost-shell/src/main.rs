use gpui::App;

fn main() {
    let app = gpui_platform::application();

    app.run(|cx: &mut App| {
        ghost_shell_app::init(cx);

        cx.activate(true);
    });
}
