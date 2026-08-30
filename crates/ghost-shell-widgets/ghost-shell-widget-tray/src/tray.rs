use std::collections::BTreeMap;

use ghost_shell_dbus::{Dbus, StatusNotifierEvent, StatusNotifierId};
use gpui::{Context, ObjectFit, Subscription, Window, div, img, prelude::*};

use crate::item::TrayItem;

mod icon;
mod item;

pub struct TrayWidget {
    items: BTreeMap<StatusNotifierId, TrayItem>,
    _subscription: Subscription,
}

impl TrayWidget {
    #[must_use]
    pub fn new(cx: &mut Context<Self>) -> Self {
        let status_notifier = cx.global::<Dbus>().status_notifier().clone();

        let items = status_notifier
            .read(cx)
            .items()
            .filter_map(|item| {
                TrayItem::try_from(item)
                    .ok()
                    .map(|tray_item| (item.id.clone(), tray_item))
            })
            .collect();

        let subscription = cx.subscribe(&status_notifier, |tray, _state, event, cx| {
            match event {
                StatusNotifierEvent::Registered(item)
                | StatusNotifierEvent::Updated(item) => {
                    let id = item.id.clone();
                    match TrayItem::try_from(item) {
                        Ok(item) => {
                            tray.items.insert(id, item);
                        }
                        Err(err) => {
                            log::error!("failed to create tray item {err:#}");
                        }
                    }
                }
                StatusNotifierEvent::Unregistered(id) => {
                    tray.items.remove(id);
                }
            }
            cx.notify();
        });

        Self {
            items,
            _subscription: subscription,
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
