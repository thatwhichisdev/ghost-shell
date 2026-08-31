use std::collections::BTreeMap;

use ghost_shell_dbus::{
    Dbus, ItemEvent, StatusNotifierId, StatusNotifierItem, WatcherEvent,
};
use gpui::{Context, ObjectFit, Subscription, Window, div, img, prelude::*};

use crate::item::TrayItem;

mod icon;
mod item;

pub struct TrayWidget {
    items: BTreeMap<StatusNotifierId, TrayItem>,
    _watcher_subscription: Subscription,
    _item_subscription: Subscription,
}

impl TrayWidget {
    #[must_use]
    pub fn new(cx: &mut Context<Self>) -> Self {
        let dbus = cx.global::<Dbus>();
        let watcher = dbus.status_notifier_watcher().clone();
        let item = dbus.status_notifier_item().clone();

        for registration in watcher
            .read(cx)
            .items()
            .map(str::to_owned)
            .collect::<Vec<_>>()
        {
            let item = item.clone();

            cx.spawn(async move |tray, cx| {
                let task = item.update(cx, |item, cx| item.discover(registration, cx));
                let status_notifier_item = task.await?;

                tray.update(cx, |tray, cx| {
                    tray.update_item(&status_notifier_item);
                    cx.notify();
                })
            })
            .detach();
        }

        let watcher_subscription = cx.subscribe(&watcher, {
            let item = item.clone();

            move |tray, _watcher, event, cx| match event {
                WatcherEvent::Registered(registration) => {
                    let reg = registration.clone();
                    let item = item.clone();

                    cx.spawn(async move |tray, cx| {
                        let task = item.update(cx, |item, cx| item.discover(reg, cx));
                        let status_notifier_item = task.await?;

                        tray.update(cx, |tray, cx| {
                            tray.update_item(&status_notifier_item);
                            cx.notify();
                        })
                    })
                    .detach();
                }

                WatcherEvent::Unregistered(registration) => {
                    let Ok(id) =
                        StatusNotifierId::from_registration(registration.as_str())
                    else {
                        return;
                    };

                    tray.items.remove(&id);
                    cx.notify();
                }
            }
        });

        let item_subscription =
            cx.subscribe(&item, |tray, _item, event, cx| match event {
                ItemEvent::Updated(item) => {
                    tray.update_item(item);
                    cx.notify();
                }
            });

        Self {
            items: BTreeMap::new(),
            _watcher_subscription: watcher_subscription,
            _item_subscription: item_subscription,
        }
    }

    fn update_item(&mut self, item: &StatusNotifierItem) {
        let id = item.id.clone();

        match TrayItem::try_from(item) {
            Ok(item) => {
                self.items.insert(id, item);
            }
            Err(error) => {
                log::warn!("failed to create tray item {id}: {error:#}");
            }
        }
    }
}

impl Render for TrayWidget {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let items: Vec<_> = self.items.iter().collect();

        div()
            .id("tray")
            .flex()
            .items_center()
            .gap_1()
            .children(items.into_iter().map(|(id, item)| {
                div()
                    .id(id.to_string())
                    .size_4()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        img(item.icon.into_inner().clone())
                            .size_4()
                            .object_fit(ObjectFit::Contain),
                    )
            }))
    }
}
