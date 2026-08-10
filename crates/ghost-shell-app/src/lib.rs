pub mod app;

pub use app::*;

use ghost_shell_config::AppConfig;
use ghost_shell_niri::NiriState;
use gpui::{App, BorrowAppContext};

pub fn init(cx: &mut App) {
    let output_focused = cx.global::<NiriState>().focused_output();
    let output_primary = cx.global::<AppConfig>().get_primary_output();

    let displays: Vec<GhostShellOutput> = cx
        .displays()
        .iter()
        .map(|display| {
            let is_focused = output_focused
                .is_some_and(|output| output == display.uuid().unwrap());
            let is_primary = output_primary == display.uuid().unwrap();

            GhostShellOutput {
                display: display.clone(),
                is_primary,
                is_focused,
            }
        })
        .collect();

    let ghost_shell = GhostShell { displays };

    cx.set_global(ghost_shell);

    // observable that will update focused display whenever it changes
    cx.observe_global::<NiriState>(|cx| {
        if let Some(uuid) = cx.global::<NiriState>().focused_output() {
            cx.update_global::<GhostShell, _>(|shell, _cx| {
                shell.set_focused_output(uuid);
            });
        };
    })
    .detach();
}
