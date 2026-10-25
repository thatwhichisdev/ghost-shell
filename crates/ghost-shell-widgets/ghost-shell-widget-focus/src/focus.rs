use ghost_shell_niri::NiriState;
use iced::{
    Element, Fill,
    widget::{container, text},
};

pub fn view<'a, Message: 'a>(niri: &'a NiriState) -> Element<'a, Message> {
    let title = niri
        .windows
        .values()
        .find(|window| window.is_focused)
        .and_then(|window| window.title.as_deref())
        .unwrap_or("");

    container(text(title).wrapping(iced::widget::text::Wrapping::None))
        .center_x(Fill)
        .clip(true)
        .into()
}
