use gpui::App;

fn main() {
    let app = gpui_platform::application();

    app.run(|cx: &mut App| match ghost_shell_app::init(cx) {
        Err(err) => {
            eprintln!("App initialization failed {err:?}");
            cx.quit();
        }
        Ok(()) => {
            println!("App initialized");
            cx.activate(true);
        }
    });
}
