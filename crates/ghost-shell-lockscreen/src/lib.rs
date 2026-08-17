mod auth;
mod lockscreen;
mod view;

use ghost_shell_actions::Lock;
use gpui::{App, BorrowAppContext as _, KeyBinding};

use crate::lockscreen::Lockscreen;

gpui::actions!(lockscreen, [Authenticate, Unlock]);

pub fn init(cx: &mut App) {
    cx.set_global(Lockscreen::new());

    cx.bind_keys([KeyBinding::new("escape", Unlock, Some("lockscreen"))]);

    cx.on_action(|_: &Lock, cx| {
        cx.update_global::<Lockscreen, _>(|lockscreen, cx| {
            if let Err(error) = lockscreen.open(cx) {
                eprintln!("Failed to lock session: {error:#}");
            }
        });
    });

    cx.on_action(|_: &Unlock, cx| {
        // Unlocking closes the session-lock windows, so defer it until the
        // current action dispatch has completed.
        cx.defer(|cx| match cx.unlock_session() {
            Ok(()) => {
                cx.update_global::<Lockscreen, _>(|lockscreen, _cx| {
                    lockscreen.clear();
                });
            }

            Err(error) => {
                eprintln!("Failed to unlock session: {error:#}");
            }
        });
    });
}
