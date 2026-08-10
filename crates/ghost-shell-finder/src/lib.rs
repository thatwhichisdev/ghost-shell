pub mod finder;
pub mod search;
pub mod view;

use ghost_shell_actions::{FinderClose, FinderToggle};
use gpui::{App, BorrowAppContext, KeyBinding};

use crate::finder::Finder;

pub fn init(cx: &mut App) {
    let finder = Finder::new();
    cx.set_global(finder);

    cx.bind_keys([KeyBinding::new("escape", FinderClose, Some("finder"))]);

    cx.on_action(|_: &FinderClose, cx| {
        cx.defer(|cx| {
            cx.update_global::<Finder, _>(|finder, cx| {
                match finder.close(cx) {
                    Ok(()) => {}
                    Err(err) => eprintln!("Failed to close launcher {err:#}"),
                }
            });
        });
    });

    cx.on_action(|_: &FinderToggle, cx| {
        cx.update_global::<Finder, _>(|finder, cx| match finder.toggle(cx) {
            Ok(()) => {}
            Err(err) => eprintln!("Failed to toggle launcher {err:#}"),
        });
    });
}
