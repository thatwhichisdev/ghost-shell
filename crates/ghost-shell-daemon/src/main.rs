use gpui::App;

fn main() {
    let app = gpui_platform::application()
        .with_assets(ghost_shell_assets::GhostShellAssets);

    app.run(|cx: &mut App| {
        gpui_tokio::init(cx);
        gpui_component::init(cx);

        if let Err(err) = ghost_shell_app::init(cx) {
            eprintln!("App initialization failed {err:#}");
            cx.quit();
        } else {
            cx.activate(true);
        }
    });
}
