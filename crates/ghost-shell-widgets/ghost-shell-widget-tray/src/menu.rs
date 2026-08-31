use ghost_shell_dbus::{Menu, MenuId, MenuItem, MenuItemType, MenuLayout};
use gpui::{App, Context, Entity, Window, px};
use gpui_component::menu::{PopupMenu, PopupMenuItem};

use crate::MENU_HEIGHT;

#[derive(Clone)]
pub(crate) struct TrayMenu {
    layout: MenuLayout,
}

impl TrayMenu {
    pub(crate) fn build(
        &self,
        dbus_menu: Entity<Menu>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<PopupMenu> {
        let menu_id = self.layout.id.clone();
        let items = self.layout.items().to_vec();

        PopupMenu::build(window, cx, move |menu, window, cx| {
            render_items(menu, &items, &menu_id, &dbus_menu, window, cx)
        })
    }
}

impl From<MenuLayout> for TrayMenu {
    fn from(layout: MenuLayout) -> Self {
        Self { layout }
    }
}

fn render_items(
    mut menu: PopupMenu,
    items: &[MenuItem],
    menu_id: &MenuId,
    dbus_menu: &Entity<Menu>,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    for item in items.iter().filter(|item| item.visible) {
        match item.item_type {
            MenuItemType::Separator => {
                menu = menu.separator();
            }

            _ if !item.children.is_empty() => {
                let children = item.children.clone();
                let menu_id = menu_id.clone();
                let dbus_menu = dbus_menu.clone();

                let submenu = PopupMenu::build(window, cx, move |submenu, window, cx| {
                    render_items(submenu, &children, &menu_id, &dbus_menu, window, cx)
                });

                menu = menu.item(
                    PopupMenuItem::submenu(item.label.clone(), submenu)
                        .disabled(!item.enabled),
                );
            }

            _ => {
                menu = menu.item(render_item(item, menu_id, dbus_menu));
            }
        }
    }

    menu
}

fn render_item(
    item: &MenuItem,
    menu_id: &MenuId,
    dbus_menu: &Entity<Menu>,
) -> PopupMenuItem {
    let item_id = item.id;
    let menu_id = menu_id.clone();
    let dbus_menu = dbus_menu.clone();

    PopupMenuItem::new(item.label.clone())
        .disabled(!item.enabled)
        .checked(item.toggle_state == 1)
        .on_click(move |_, _, cx| {
            let task = dbus_menu
                .update(cx, |menu, cx| menu.activate(menu_id.clone(), item_id, cx));

            cx.spawn(async move |_| {
                if let Err(error) = task.await {
                    log::warn!(
                        "failed to activate D-Bus menu item: \
                         {error:#}"
                    );
                }
            })
            .detach();
        })
}
