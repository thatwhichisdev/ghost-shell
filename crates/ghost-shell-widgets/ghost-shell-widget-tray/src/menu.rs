use ghost_shell_dbus::{Dbus, Menu, MenuId, MenuItem, MenuItemType, MenuLayout};
use gpui::{App, Context, Entity, Pixels, Size, Window, px, size};
use gpui_component::menu::{PopupMenu, PopupMenuItem};

const MENU_WIDTH: f32 = 250.0;
const MENU_ITEM_HEIGHT: f32 = 26.0;
const MENU_SEPARATOR_HEIGHT: f32 = 6.0;
const MENU_GAP: f32 = 2.0;
const MENU_INSET: f32 = 5.0;

#[derive(Clone)]
pub(crate) struct TrayMenu {
    layout: MenuLayout,
}

impl TrayMenu {
    pub(crate) fn build(&self, window: &mut Window, cx: &mut App) -> Entity<PopupMenu> {
        let menu_id = self.layout.id.clone();
        let items = self.layout.items().to_vec();

        PopupMenu::build(window, cx, move |menu, window, cx| {
            render_items(menu, &items, &menu_id, window, cx)
        })
    }

    pub(crate) fn size(&self) -> Size<Pixels> {
        let metrics = measure(self.layout.items());
        size(px(MENU_WIDTH * metrics.depth as f32), px(metrics.height))
    }
}

impl From<MenuLayout> for TrayMenu {
    fn from(layout: MenuLayout) -> Self {
        Self { layout }
    }
}

struct MenuMetrics {
    depth: usize,
    height: f32,
}

fn measure(items: &[MenuItem]) -> MenuMetrics {
    let mut depth = 1;
    let mut y = MENU_INSET;
    let mut height = MENU_INSET * 2.0;

    for item in items.iter().filter(|item| item.visible) {
        if !item.children.is_empty() {
            let child = measure(&item.children);

            depth = depth.max(child.depth + 1);
            height = height.max(y + child.height);
        }

        y += match item.item_type {
            MenuItemType::Separator => MENU_SEPARATOR_HEIGHT,
            _ => MENU_ITEM_HEIGHT,
        };

        y += MENU_GAP;
        height = height.max(y + MENU_INSET);
    }

    MenuMetrics { depth, height }
}

fn render_items(
    mut menu: PopupMenu,
    items: &[MenuItem],
    menu_id: &MenuId,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let dbus_menu = cx.global::<Dbus>().dbus_menu().clone();

    for item in items.iter().filter(|item| item.visible) {
        menu = match item.item_type {
            MenuItemType::Separator => menu.separator(),

            _ if !item.children.is_empty() => {
                let children = item.children.clone();
                let menu_id = menu_id.clone();

                let submenu = PopupMenu::build(window, cx, move |menu, window, cx| {
                    render_items(menu, &children, &menu_id, window, cx)
                });

                menu.item(
                    PopupMenuItem::submenu(item.label.clone(), submenu)
                        .disabled(!item.enabled),
                )
            }

            _ => menu.item(render_item(item, menu_id, &dbus_menu)),
        };
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
                    log::warn!("failed to activate D-Bus menu item: {error:#}");
                }
            })
            .detach();
        })
}
