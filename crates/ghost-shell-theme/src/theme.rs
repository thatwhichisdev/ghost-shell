use ghost_shell_config::Base16Config;
use gpui::{Hsla, rgb};
use gpui_component::Theme;

pub fn apply_base16(theme: &mut Theme, palette: &Base16Config) {
    let base00 = color(palette.base00);
    let base01 = color(palette.base01);
    let base02 = color(palette.base02);
    let base03 = color(palette.base03);
    let base04 = color(palette.base04);
    let base05 = color(palette.base05);
    let base06 = color(palette.base06);
    let base07 = color(palette.base07);
    let base08 = color(palette.base08);
    let base09 = color(palette.base09);
    let base0a = color(palette.base0a);
    let base0b = color(palette.base0b);
    let base0c = color(palette.base0c);
    let base0d = color(palette.base0d);
    let base0e = color(palette.base0e);
    let base0f = color(palette.base0f);

    // Main surfaces.
    theme.colors.background = base00;
    theme.colors.foreground = base05;

    // Subdued surfaces and text.
    theme.colors.muted = base01;
    theme.colors.muted_foreground = base03;

    theme.colors.secondary = base01;
    theme.colors.secondary_foreground = base05;
    theme.colors.secondary_hover = base02;
    theme.colors.secondary_active = base02;

    // Borders and inputs.
    theme.colors.border = base02;
    theme.colors.input = base01;
    theme.colors.ring = base0d;

    // Popovers.
    theme.colors.popover = base00;
    theme.colors.popover_foreground = base05;

    // Selection / list states.
    theme.colors.accent = base01;
    theme.colors.accent_foreground = base05;

    theme.colors.selection = base02;

    theme.colors.list = base00;
    theme.colors.list_hover = base01;
    theme.colors.list_active = base02;
    theme.colors.list_active_border = base03;

    // Primary accent.
    theme.colors.primary = base0d;
    theme.colors.primary_foreground = base00;

    // Links / focus-like accent.
    theme.colors.link = base0d;

    // Semantic states.
    theme.colors.danger = base08;
    theme.colors.warning = base0a;
    theme.colors.success = base0b;
    theme.colors.info = base0d;

    // Generic named colors used by gpui-component.
    theme.colors.red = base08;
    theme.colors.yellow = base0a;
    theme.colors.green = base0b;
    theme.colors.cyan = base0c;
    theme.colors.blue = base0d;
    theme.colors.magenta = base0e;

    // Keep tokens synchronized with ThemeColor.
    theme.tokens = (&theme.colors).into();

    let _ = (base04, base06, base07, base09, base0f);
}

fn color(value: u32) -> Hsla {
    rgb(value).into()
}
