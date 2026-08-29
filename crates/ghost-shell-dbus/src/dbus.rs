mod notifications;
mod status_notifier;
mod status_notifier_item;
mod status_notifier_watcher;

use gpui::{App, Entity, Global};

pub use status_notifier::{StatusNotifierEvent, StatusNotifierState};

pub struct Dbus {
    status_notifier: Entity<StatusNotifierState>,
}

impl Dbus {
    #[must_use]
    pub fn status_notifier(&self) -> &Entity<StatusNotifierState> {
        &self.status_notifier
    }
}

impl Global for Dbus {}

pub fn init(cx: &mut App) {
    let status_notifier = status_notifier::init(cx);
    let dbus = Dbus { status_notifier };

    cx.set_global(dbus);
}
