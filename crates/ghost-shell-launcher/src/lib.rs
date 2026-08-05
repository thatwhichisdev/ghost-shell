pub mod applications;
pub mod launcher;

pub use applications::*;
pub use launcher::*;

use gpui::{App, BorrowAppContext as _, KeyBinding};

use ghost_shell_actions::{CloseLauncher, ToggleLauncher};

pub fn init(cx: &mut App) {
    let apps = applications::load();
    cx.set_global(apps);

    let launcher = Launcher::new();
    cx.set_global(launcher);

    cx.bind_keys([KeyBinding::new("escape", CloseLauncher, Some("launcher"))]);

    cx.on_action(|_: &CloseLauncher, cx| {
        cx.defer(|cx| {
            cx.update_global::<Launcher, _>(|launcher, cx| {
                match launcher.close(cx) {
                    Ok(()) => {}
                    Err(err) => eprintln!("Failed to close launcher {err:#}"),
                }
            });
        });
    });

    cx.on_action(|_: &ToggleLauncher, cx| {
        cx.update_global::<Launcher, _>(|launcher, cx| {
            match launcher.toggle(cx) {
                Ok(()) => {}
                Err(err) => eprintln!("Failed to toggle launcher {err:#}"),
            }
        });
    });
}
