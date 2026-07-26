use gpui::App;

fn main() {
    let app = gpui_platform::application();

    app.run(|cx: &mut App| {
        if let Err(err) = ghost_shell_app::init(cx) {
            eprintln!("App initialization failed {err:#}");
            cx.quit();
        } else {
            println!("App initialized");
            cx.activate(true);
        }
    });
}
