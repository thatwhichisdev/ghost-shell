use iced::{Element, widget::button};

#[derive(Debug, Clone)]
pub enum Message {
    OpenLauncher,
}

pub fn view(style: ghost_shell_theme::bar::Style) -> Element<'static, Message> {
    ghost_shell_theme::bar::outer(
        button(ghost_shell_theme::bar::inner(
            ghost_shell_theme::bar::icon("icons/nixos.svg", style.widgets.icon_size),
            style,
        ))
        .padding(0)
        .on_press(Message::OpenLauncher)
        .style(ghost_shell_theme::bar::transparent_button),
        style,
    )
}
