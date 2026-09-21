fn main() {
    let env = env_logger::Env::default().default_filter_or("info");

    env_logger::Builder::from_env(env).init();

    let app = ghost_shell_gpui::application()
        .with_quit_mode(ghost_shell_gpui::QuitMode::Explicit)
        .with_assets(ghost_shell_assets::Assets);

    app.run(|cx: &mut ghost_shell_gpui::App| {
        ghost_shell_tokio::init(cx);
        ghost_shell_component_root::init(cx);
        ghost_shell_component_input::init(cx);
        ghost_shell_component_menu::init(cx);

        ghost_shell_config::init(cx);
        configure_theme(cx);

        ghost_shell_dbus::init(cx);
        ghost_shell_niri::init(cx);
        ghost_shell_ipc::init(cx);

        ghost_shell_app::init(cx);

        ghost_shell_wallpaper::init(cx);
        ghost_shell_lockscreen::init(cx);
        ghost_shell_launcher::init(cx);
        ghost_shell_finder::init(cx);
        ghost_shell_bar::init(cx);

        cx.activate(true);
    });
}

fn configure_theme(cx: &mut ghost_shell_gpui::App) {
    use ghost_shell_config::{AppConfig, ThemeMode as ConfigThemeMode};
    use ghost_shell_theme::{Theme, ThemeMode};

    let config = cx.global::<AppConfig>();
    let mode = match config.theme.mode {
        ConfigThemeMode::Dark => ThemeMode::Dark,
        ConfigThemeMode::Light => ThemeMode::Light,
        ConfigThemeMode::System => cx.window_appearance().into(),
    };
    let palette = match mode {
        ThemeMode::Dark => &config.theme.dark,
        ThemeMode::Light => &config.theme.light,
    };
    let mut theme = Theme::new(mode);
    theme.tokens.typography.sans = config.general.font_family.clone().into();
    theme.tokens.typography.md.size = ghost_shell_gpui::px(config.general.font_size);
    theme.apply_base16(&[
        palette.base00,
        palette.base01,
        palette.base02,
        palette.base03,
        palette.base04,
        palette.base05,
        palette.base06,
        palette.base07,
        palette.base08,
        palette.base09,
        palette.base0a,
        palette.base0b,
        palette.base0c,
        palette.base0d,
        palette.base0e,
        palette.base0f,
    ]);
    Theme::set(theme, cx);
}
