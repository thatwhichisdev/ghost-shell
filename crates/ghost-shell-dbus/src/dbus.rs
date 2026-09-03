mod item;
mod menu;
mod notifications;
mod watcher;

use gpui::{App, Entity, Global};

use crate::{item::Item, watcher::Watcher};
pub use crate::{
    item::{IconPixmap, ItemEvent, StatusNotifierId, StatusNotifierItem},
    menu::{Menu, MenuId, MenuItem, MenuItemType, MenuLayout},
    watcher::WatcherEvent,
};

pub struct Dbus {
    status_notifier_watcher: Entity<Watcher>,
    status_notifier_item: Entity<Item>,
    dbus_menu: Entity<Menu>,
}

impl Dbus {
    #[must_use]
    pub fn status_notifier_watcher(&self) -> &Entity<Watcher> {
        &self.status_notifier_watcher
    }

    #[must_use]
    pub fn status_notifier_item(&self) -> &Entity<Item> {
        &self.status_notifier_item
    }

    #[must_use]
    pub fn dbus_menu(&self) -> &Entity<Menu> {
        &self.dbus_menu
    }
}

impl Global for Dbus {}

pub fn init(cx: &mut App) {
    let status_notifier_watcher = watcher::init(cx);
    let status_notifier_item = item::init(cx);
    let dbus_menu = menu::init(cx);

    let dbus = Dbus {
        status_notifier_watcher,
        status_notifier_item,
        dbus_menu,
    };

    cx.set_global(dbus);
}
