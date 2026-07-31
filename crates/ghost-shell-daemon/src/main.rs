use gpui::App;

/// Entry point for the daemon.
///
/// Initializes shell application assets, required to load *.svg icons.
///
/// Initializes tokio runtime using `gpui_tokio`,
/// required to for inter-components communication using synchronization primitives,
/// also used to create async IPC server and clients.
///
/// Initializes gpui-component components using `gpui_component`,
/// required for building UI compontens.
///
/// Initializes shell application itself, which loads bars and initializes all widgets.
///
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
