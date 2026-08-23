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
/// Initializes niri ipc client and event stream,
/// required niri related widgets to send events to niri and also receive niri state over event stream.
///
/// Initializes shell application itself, which loads bars and initializes all widgets.
///
fn main() {
    let env = env_logger::Env::default().default_filter_or("info");

    env_logger::Builder::from_env(env).init();

    let app = gpui_platform::application().with_assets(ghost_shell_assets::Assets);

    app.run(|cx: &mut gpui::App| {
        gpui_tokio::init(cx);
        gpui_component::init(cx);

        ghost_shell_config::init(cx);
        ghost_shell_wallpaper::init(cx);
        ghost_shell_theme::init(cx);
        ghost_shell_niri::init(cx);
        ghost_shell_ipc::init(cx);
        ghost_shell_app::init(cx);
        ghost_shell_lockscreen::init(cx);
        ghost_shell_launcher::init(cx);
        ghost_shell_finder::init(cx);
        ghost_shell_bar::init(cx);

        cx.activate(true);
    });
}
