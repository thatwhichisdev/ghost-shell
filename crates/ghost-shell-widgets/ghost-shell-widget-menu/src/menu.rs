use iced::{
    Element, Fill,
    widget::{button, column, container, row, sensor, text},
};

#[derive(Debug, Clone)]
pub enum Message {
    Open,
    Resize(u32, u32),
    Shutdown,
    Reboot,
    Lock,
    Styling,
}

pub fn view(style: ghost_shell_theme::bar::Style) -> Element<'static, Message> {
    ghost_shell_theme::bar::outer(
        button(ghost_shell_theme::bar::inner(
            ghost_shell_theme::bar::icon("icons/nixos.svg", style.widgets.icon_size),
            style,
        ))
        .padding(0)
        .on_press(Message::Open)
        .style(ghost_shell_theme::bar::transparent_button),
        style,
    )
}

pub fn popup<'a>(
    error: Option<&'a str>,
    busy: bool,
    configuration_error: bool,
    font: iced::Font,
    font_size: f32,
) -> Element<'a, Message> {
    let mut items = column![];
    for (label, icon, message) in [
        ("Shutdown", "icons/power-fill.svg", Message::Shutdown),
        ("Reboot", "icons/arrow-clockwise-fill.svg", Message::Reboot),
        ("Lock", "icons/lock-key-fill.svg", Message::Lock),
        (
            if configuration_error {
                "Appearance!"
            } else {
                "Appearance"
            },
            "icons/layout-fill.svg",
            Message::Styling,
        ),
    ] {
        items = items.push(
            container(
                button(
                    row![
                        container(ghost_shell_theme::bar::icon(icon, 16.0)).center_y(22),
                        ghost_shell_theme::label(label, font, font_size).width(Fill)
                    ]
                    .spacing(6)
                    .align_y(iced::Center),
                )
                .width(Fill)
                .padding([0, 5])
                .style(ghost_shell_theme::menu_item)
                .on_press_maybe((!busy).then_some(message)),
            )
            .width(Fill)
            .padding(4),
        );
    }
    if busy {
        items = items.push(ghost_shell_theme::label("Please wait…", font, font_size));
    }
    if let Some(error) = error {
        items = items
            .push(ghost_shell_theme::label(error, font, font_size).style(text::danger));
    }
    let measured = sensor(items)
        .on_show(menu_size)
        .on_resize(menu_size);
    container(iced::widget::scrollable(measured))
        .padding(4)
        .width(Fill)
        .height(Fill)
        .style(ghost_shell_theme::menu_surface)
        .into()
}

fn menu_size(size: iced::Size) -> Message {
    Message::Resize(
        258,
        (size.height.ceil() as u32)
            .saturating_add(8)
            .clamp(38, 702),
    )
}

pub fn initial_size(font_size: f32) -> (u32, u32) {
    (
        258,
        ((font_size * 1.3).max(22.0).ceil() as u32 + 8)
            .saturating_mul(4)
            .saturating_add(8)
            .min(702),
    )
}
