pub mod app;

pub use app::*;
use ghost_shell_actions::ToggleLauncher;
use ghost_shell_config::AppConfig;
use ghost_shell_niri::NiriState;

use std::collections::HashMap;

use gpui::{App, Entity, accesskit::Uuid, prelude::*};

use ghost_shell_bar::{Bar, Widgets};
use ghost_shell_launcher::Launcher;
use ghost_shell_widget_clock::ClockWidget;
use ghost_shell_widget_focus::FocusWidget;
use ghost_shell_widget_menu::MenuWidget;
use ghost_shell_widget_power::PowerWidget;
use ghost_shell_widget_workspaces::WorkspacesWidget;

/// Loads app configuration and opens bars on available displays.
///
/// # Panics
/// Panics when app initialization fails.
///
pub fn init(cx: &mut App) {
    let config = cx.global::<AppConfig>().clone();

    let launcher = cx.new(|_cx| Launcher::default());
    let menu = cx.new(|_cx| MenuWidget {});
    let power = cx.new(|_cx| PowerWidget {});
    let clock = cx.new(ClockWidget::new);
    let focus = cx.new(FocusWidget::new);

    let bars: HashMap<Uuid, Entity<Bar>> = config
        .bars
        .into_iter()
        .map(|(output, bar_config)| {
            let id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, output.as_bytes());
            let display = cx
                .displays()
                .iter()
                .find(|display| display.uuid().is_ok_and(|uuid| uuid == id))
                .cloned()
                .unwrap(); // for now display should be always present in the config, will change later

            let workspaces = cx.new(|cx| WorkspacesWidget::new(cx, id));
            let widgets = Widgets {
                menu: menu.clone(),
                workspaces,
                focus: focus.clone(),
                power: power.clone(),
                clock: clock.clone(),
            };

            let bar = Bar::new(cx, bar_config, widgets, display.clone());

            bar.update(cx, |bar, cx| {
                bar.open(cx);
            });

            (display.uuid().unwrap(), bar)
        })
        .collect();

    let shell = GhostShell::new(launcher, bars);
    cx.set_global(shell);

    cx.on_action(|_: &ToggleLauncher, cx| {
        cx.update_global::<GhostShell, _>(|shell, cx| {
            cx.update_entity::<Launcher, _>(&shell.launcher, |launcher, cx| {
                let niri_state = cx.global::<NiriState>();
                let id = niri_state
                    .clone()
                    .workspaces
                    .into_values()
                    .find(|workspace| workspace.is_focused == true)
                    .and_then(|workspace| workspace.output)
                    .map(|output| {
                        Uuid::new_v5(&Uuid::NAMESPACE_DNS, output.as_bytes())
                    })
                    .unwrap(); // for now panic, but ideally we should toggle launcher on primary output if nothing is focused

                let display = cx
                    .displays()
                    .iter()
                    .find(|display| display.uuid().is_ok_and(|uuid| uuid == id))
                    .cloned()
                    .unwrap(); // for now display should be always present in the config, will change later

                let _ = launcher.toggle(cx, &display);
            });
        });
    });

    cx.activate(true);
}
