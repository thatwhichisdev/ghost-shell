pub mod app;

pub use app::*;
use ghost_shell_config::AppConfig;
use ghost_shell_gpui::{App, BorrowAppContext, accesskit::Uuid};
use ghost_shell_niri::NiriState;

pub fn init(cx: &mut App) {
    cx.set_global(GhostShell::new(Vec::new()));
    refresh_displays(cx);
    cx.on_displays_changed(refresh_displays)
        .detach();

    cx.observe_global::<NiriState>(|cx| {
        let focused = cx.global::<NiriState>().focused_output();
        cx.update_global::<GhostShell, _>(|shell, _cx| {
            shell.set_focused_output(focused);
        });
    })
    .detach();
}

fn refresh_displays(cx: &mut App) {
    let output_focused = cx.global::<NiriState>().focused_output();
    let output_primary = cx
        .global::<AppConfig>()
        .bars
        .iter()
        .filter(|(_output, bar)| bar.primary)
        .map(|(output, _bar)| output)
        .min()
        .map(|output| Uuid::new_v5(&Uuid::NAMESPACE_DNS, output.as_bytes()));
    let displays = cx.displays();
    cx.update_global::<GhostShell, _>(|shell, _cx| {
        shell.set_displays(displays, output_primary, output_focused);
    });
}
