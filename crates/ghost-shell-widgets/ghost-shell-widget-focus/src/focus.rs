use ghost_shell_niri::NiriState;
use iced::{
    Element,
    widget::{container, text},
};

pub fn view<'a, Message: 'a>(
    niri: &'a NiriState,
    style: ghost_shell_theme::bar::Style,
) -> Element<'a, Message> {
    let title = niri
        .windows
        .values()
        .find(|window| window.is_focused)
        .and_then(|window| window.title.as_deref())
        .unwrap_or("");

    if title.is_empty() {
        return iced::widget::Space::new()
            .width(0)
            .height(0)
            .into();
    }
    ghost_shell_theme::bar::outer(
        ghost_shell_theme::bar::inner(
            container(
                text(title)
                    .font(style.font)
                    .size(style.font_size)
                    .wrapping(iced::widget::text::Wrapping::None),
            )
            .clip(true),
            style,
        ),
        style,
    )
}
