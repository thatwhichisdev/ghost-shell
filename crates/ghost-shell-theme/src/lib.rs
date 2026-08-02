pub mod theme;

use gpui::{App, Hsla, UpdateGlobal, px, rgba};

use ghost_shell_config::AppConfig;
use gpui_component::{Theme, ThemeMode};

pub fn init(cx: &mut App) {
    let config = cx.global::<AppConfig>().clone();

    Theme::change(ThemeMode::Dark, None, cx);

    let background: Hsla = rgba(config.general.bg).into();
    let foreground: Hsla = rgba(config.general.fg).into();

    Theme::update_global(cx, |theme, _cx| {
        theme.font_family = config.general.font_family.clone().into();
        theme.font_size = px(config.general.font_size);

        theme.colors.background = background;
        theme.colors.foreground = foreground;

        theme.tokens.background = background.into();
        theme.tokens.foreground = foreground.into();
    });
}
