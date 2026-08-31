use std::collections::BTreeMap;

use ghost_shell_dbus::{
    Dbus, ItemEvent, MenuId, StatusNotifierId, StatusNotifierItem, WatcherEvent,
};
use gpui::{
    AppContext as _, Bounds, Context, DismissEvent, MouseDownEvent, ObjectFit, Pixels,
    Point, Subscription, VisualContext as _, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, img, point,
    popup::{PopupAnchor, PopupConstraintAdjustment, PopupGravity, PopupOptions},
    prelude::*,
    px, size,
};

use crate::{item::TrayItem, menu::TrayMenu};

mod icon;
mod item;
mod menu;

const MENU_WIDTH: Pixels = px(260.0);
const MENU_HEIGHT: Pixels = px(320.0);

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
            Self::discover(registration, cx);
        }

        let watcher_subscription =
            cx.subscribe(&watcher, |tray, _watcher, event, cx| match event {
                WatcherEvent::Registered(registration) => {
                    Self::discover(registration.clone(), cx);
                }

                WatcherEvent::Unregistered(registration) => {
                    let Ok(id) = StatusNotifierId::from_registration(registration) else {
                        return;
                    };

                    tray.items.remove(&id);
                    cx.notify();
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

    fn discover(registration: String, cx: &mut Context<Self>) {
        let dbus = cx.global::<Dbus>();

        let item = dbus.status_notifier_item().clone();
        let menu = dbus.dbus_menu().clone();

        cx.spawn(async move |tray, cx| {
            let item_task = item.update(cx, |item, cx| item.discover(registration, cx));

            let status_notifier_item = match item_task.await {
                Ok(item) => item,

                Err(error) => {
                    log::warn!("failed to discover status notifier item: {error:#}");

                    return;
                }
            };

            let tray_menu = if let Some(path) = status_notifier_item.menu.as_deref() {
                let id = MenuId::new(status_notifier_item.id.service(), path);

                let menu_task = menu.update(cx, |menu, cx| menu.discover(id, cx));

                match menu_task.await {
                    Ok(layout) => Some(TrayMenu::from(layout)),

                    Err(error) => {
                        log::warn!(
                            "failed to discover tray menu for {}: \
                                 {error:#}",
                            status_notifier_item.id
                        );

                        None
                    }
                }
            } else {
                None
            };

            _ = tray.update(cx, |tray, cx| {
                tray.insert_item(&status_notifier_item, tray_menu);

                cx.notify();
            });
        })
        .detach();
    }

    fn insert_item(&mut self, item: &StatusNotifierItem, menu: Option<TrayMenu>) {
        let id = item.id.clone();

        match TrayItem::new(item, menu) {
            Ok(item) => {
                self.items.insert(id, item);
            }

            Err(error) => {
                log::warn!("failed to create tray item {id}: {error:#}");
            }
        }
    }

    fn update_item(&mut self, item: &StatusNotifierItem) {
        let id = &item.id;

        let Some(tray_item) = self.items.get_mut(id) else {
            match TrayItem::try_from(item) {
                Ok(item) => {
                    self.items.insert(id.clone(), item);
                }

                Err(error) => {
                    log::warn!("failed to create tray item {id}: {error:#}");
                }
            }

            return;
        };

        if let Err(error) = tray_item.update(item) {
            log::warn!("failed to update tray item {id}: {error:#}");
        }
    }

    fn open_menu(
        &mut self,
        menu: TrayMenu,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let popup = PopupOptions {
            parent: window.window_handle(),

            // Mouse coordinates are already local to the bar surface.
            anchor_rect: Bounds::new(position, size(px(1.0), px(1.0))),

            // Tray is on the right, so grow down and towards the left.
            anchor: PopupAnchor::BottomRight,
            gravity: PopupGravity::BottomLeft,

            constraint_adjustment: PopupConstraintAdjustment::SLIDE_X
                | PopupConstraintAdjustment::SLIDE_Y
                | PopupConstraintAdjustment::FLIP_X
                | PopupConstraintAdjustment::FLIP_Y,

            offset: point(px(0.0), px(4.0)),

            // Correct behavior for a context menu.
            grab: true,
        };

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                Default::default(),
                size(MENU_WIDTH, MENU_HEIGHT),
            ))),
            kind: WindowKind::AnchoredPopup(popup),
            titlebar: None,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            window_background: WindowBackgroundAppearance::Transparent,
            ..Default::default()
        };

        if let Err(error) = cx.open_window(options, move |window, cx| {
            let dbus_menu = cx.global::<Dbus>().dbus_menu().clone();
            let popup_menu = menu.build(dbus_menu, window, cx);

            // PopupMenu normally emits DismissEvent and its ContextMenu
            // wrapper hides it. We don't use that wrapper anymore, so close
            // the native popup window ourselves.
            window
                .subscribe(&popup_menu, cx, |_menu, _: &DismissEvent, window, _cx| {
                    window.remove_window();
                })
                .detach();

            popup_menu
        }) {
            log::warn!("failed to open tray menu: {error:#}");
        }
    }
}

impl Render for TrayWidget {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id("tray")
            .flex()
            .items_center()
            .gap_1()
            .children(self.items.iter().map(|(id, item)| {
                let menu = item.menu.clone();

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
                    .on_mouse_down(
                        gpui::MouseButton::Right,
                        cx.listener(move |tray, event: &MouseDownEvent, window, cx| {
                            let Some(menu) = menu.clone() else {
                                return;
                            };

                            tray.open_menu(menu, event.position, window, cx);
                        }),
                    )
            }))
    }
}
