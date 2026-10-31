use iced::{Element, widget::button};

#[derive(Debug, Clone)]
pub enum Message {
    Toggle,
}

pub fn view(style: ghost_shell_theme::bar::Style) -> Element<'static, Message> {
    ghost_shell_theme::bar::outer(
        button(ghost_shell_theme::bar::inner(
            ghost_shell_theme::bar::icon(
                "icons/circle-half-fill.svg",
                style.widgets.icon_size,
            ),
            style,
        ))
        .padding(0)
        .on_press(Message::Toggle)
        .style(ghost_shell_theme::bar::transparent_button),
        style,
    )
}
