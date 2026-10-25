use ghost_shell_config::{ThemeConfig, ThemeMode};
use iced::{
    Color, Element, Fill, Theme,
    widget::{Space, column, container, scrollable},
};

pub fn color(value: u32) -> Color {
    Color::from_rgb8((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

pub fn rgba(value: u32) -> Color {
    let mut color = color(value >> 8);
    color.a = (value & 255) as f32 / 255.0;
    color
}

pub fn theme(config: &ThemeConfig, system_dark: bool) -> Theme {
    let palette = match config.mode {
        ThemeMode::Dark => &config.dark,
        ThemeMode::Light => &config.light,
        ThemeMode::System if system_dark => &config.dark,
        ThemeMode::System => &config.light,
    };
    Theme::custom(
        "Ghost".to_owned(),
        iced::theme::Palette {
            background: color(palette.base00),
            text: color(palette.base05),
            primary: color(palette.base0d),
            success: color(palette.base0b),
            warning: color(palette.base0a),
            danger: color(palette.base08),
        },
    )
}

pub fn surface(theme: &Theme, opacity: f32) -> container::Style {
    let mut background = theme.palette().background;
    background.a = opacity;
    container::Style::default()
        .background(background)
        .color(theme.palette().text)
}

pub fn selected(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::primary(theme, status)
}

pub fn plain(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::text(theme, status)
}

pub fn virtual_list<'a, Message: Clone + 'a>(
    count: usize,
    offset: f32,
    viewport_height: f32,
    row_height: f32,
    id: &'static str,
    on_scroll: impl Fn(scrollable::Viewport) -> Message + 'a,
    mut row: impl FnMut(usize) -> Element<'a, Message>,
) -> Element<'a, Message> {
    let start = ((offset.max(0.0) / row_height) as usize)
        .saturating_sub(1)
        .min(count);
    let end = (start + (viewport_height / row_height).ceil() as usize + 3).min(count);
    let mut content = column![Space::new().height(start as f32 * row_height)];
    for index in start..end {
        content = content.push(
            container(row(index))
                .height(row_height)
                .width(Fill)
                .clip(true),
        );
    }
    content = content.push(Space::new().height((count - end) as f32 * row_height));
    scrollable(content)
        .id(id)
        .on_scroll(on_scroll)
        .height(Fill)
        .into()
}

pub fn selection(current: Option<usize>, count: usize, forward: bool) -> Option<usize> {
    if count == 0 {
        return None;
    }
    Some(match current.filter(|index| *index < count) {
        None => 0,
        Some(index) if forward => (index + 1) % count,
        Some(0) => count - 1,
        Some(index) => index - 1,
    })
}
