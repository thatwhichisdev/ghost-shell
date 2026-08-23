pub mod theme;

use gpui::{App, UpdateGlobal, px};

use ghost_shell_config::AppConfig;
use gpui_component::{Theme, ThemeMode};

pub fn init(cx: &mut App) {
    let (font_family, font_size, mode, palette) = {
        let config = cx.global::<AppConfig>();

        let (mode, palette) = match config.theme.mode {
            ghost_shell_config::ThemeMode::Dark => {
                (ThemeMode::Dark, config.theme.dark.clone())
            }
            ghost_shell_config::ThemeMode::Light => {
                (ThemeMode::Light, config.theme.light.clone())
            }
            ghost_shell_config::ThemeMode::System => todo!(),
        };

        (
            config.general.font_family.clone(),
            config.general.font_size,
            mode,
            palette,
        )
    };

    Theme::change(mode, None, cx);
    Theme::update_global(cx, |theme, _| {
        theme.font_family = font_family.into();
        theme.font_size = px(font_size);

        theme::apply_base16(theme, &palette);
    });
}
