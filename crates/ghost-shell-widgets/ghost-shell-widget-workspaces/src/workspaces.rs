use ghost_shell_niri::NiriState;
use iced::{Element, widget::row};

pub fn view<'a, Message: 'a>(
    output: &str,
    niri: &'a NiriState,
    style: ghost_shell_theme::bar::Style,
) -> Element<'a, Message> {
    let mut workspaces: Vec<_> = niri
        .workspaces
        .values()
        .filter(|workspace| workspace.output.as_deref() == Some(output))
        .collect();

    workspaces.sort_by_key(|workspace| workspace.idx);

    if workspaces.is_empty() {
        return iced::widget::Space::new()
            .width(0)
            .height(0)
            .into();
    }
    let workspaces = row(workspaces.into_iter().map(|workspace| {
        ghost_shell_theme::bar::icon(
            if workspace.is_active {
                "icons/circle-filled.svg"
            } else {
                "icons/circle.svg"
            },
            style.widgets.icon_size,
        )
    }))
    .spacing(4)
    .align_y(iced::Center);

    ghost_shell_theme::bar::outer(ghost_shell_theme::bar::inner(workspaces, style), style)
}
