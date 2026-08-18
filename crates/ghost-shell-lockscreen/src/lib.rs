mod auth;
mod lockscreen;
mod view;

use ghost_shell_actions::Lock;
use gpui::{App, BorrowAppContext as _};

use crate::lockscreen::LockManager;

gpui::actions!(lockscreen, [Authenticate, Unlock]);

pub fn init(cx: &mut App) {
    cx.set_global(LockManager::new());

    // Global listener for the `Lock` event which is disptached by IPC channel
    cx.on_action(|_: &Lock, cx| {
        match cx.update_global::<LockManager, _>(|lock_manager, cx| {
            lock_manager.lock(cx)
        }) {
            Ok(()) => log::info!("Locked the session"),
            Err(e) => log::error!("Locking the session failed {e}"),
        }
    });

    // Local listener for the 'Unlock' event which is dispatched after successful pam authorization
    cx.on_action(|_: &Unlock, cx| {
        match cx.update_global::<LockManager, _>(|lock_manager, cx| {
            lock_manager.unlock(cx)
        }) {
            Ok(()) => log::info!("Unlocked the session"),
            Err(e) => log::error!("Unlocking the session failed {e}"),
        }
    });
}
