//! Session locking and authentication for Ghost-Shell.
//!
//! This crate creates session-lock windows for active displays, renders the
//! lock screen, and authenticates the current user through PAM.
//!
//! Call [`init`] once during application startup;
mod auth;
mod lockscreen;
mod view;

use ghost_shell_actions::Lock;
use gpui::{App, BorrowAppContext as _};

use crate::lockscreen::LockManager;

gpui::actions!(lockscreen, [Authenticate, Unlock]);

/// Initializes lockscreen state and action handlers.
pub fn init(cx: &mut App) {
    cx.set_global(LockManager::new());

    cx.on_action(|_: &Lock, cx| {
        match cx.update_global::<LockManager, _>(|lock_manager, cx| lock_manager.lock(cx))
        {
            Ok(()) => log::debug!("Locked the session"),
            Err(e) => log::error!("Locking the session failed {e}"),
        }
    });

    cx.on_action(|_: &Unlock, cx| {
        match cx
            .update_global::<LockManager, _>(|lock_manager, cx| lock_manager.unlock(cx))
        {
            Ok(()) => log::debug!("Unlocked the session"),
            Err(e) => log::error!("Unlocking the session failed {e}"),
        }
    });
}
