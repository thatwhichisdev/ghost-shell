use iced::{
    Element, Fill,
    widget::{button, container},
};

#[derive(Debug, Clone)]
pub enum Message {
    OpenLauncher,
}

pub fn view() -> Element<'static, Message> {
    button(container(ghost_shell_assets::icon("icons/nixos.svg", 18.0)).center_y(Fill))
        .padding(0)
        .height(Fill)
        .on_press(Message::OpenLauncher)
        .style(button::text)
        .into()
}
