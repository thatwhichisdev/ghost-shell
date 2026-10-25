use ghost_shell_niri::NiriState;
use iced::{Element, widget::row};

pub fn view<'a, Message: 'a>(output: &str, niri: &'a NiriState) -> Element<'a, Message> {
    let mut workspaces: Vec<_> = niri
        .workspaces
        .values()
        .filter(|workspace| workspace.output.as_deref() == Some(output))
        .collect();

    workspaces.sort_by_key(|workspace| workspace.idx);

    let workspaces = row(workspaces.into_iter().map(|workspace| {
        ghost_shell_assets::icon(
            if workspace.is_active {
                "icons/circle-filled.svg"
            } else {
                "icons/circle.svg"
            },
            18.0,
        )
    }))
    .spacing(4)
    .align_y(iced::Center);

    workspaces.into()
}
