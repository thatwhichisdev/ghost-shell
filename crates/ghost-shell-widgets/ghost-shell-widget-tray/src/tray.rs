use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use ghost_shell_dbus::{
    MenuId, MenuItem, MenuItemType, MenuLayout, StatusNotifierId, StatusNotifierItem,
};
use iced::{
    Color, ContentFit, Element, Fill,
    widget::{button, container, row, text},
};
mod menu_popup;

#[derive(Clone, Debug)]
pub enum TrayMessage {
    Open(MenuId),
    Dismiss,
    Resize(u32, u32),
    Activate(i32),
}

enum Icon {
    Svg(iced::widget::svg::Handle),
    Image(iced::widget::image::Handle),
}

struct Item {
    icon: Option<Icon>,
    menu: Option<MenuId>,
}

#[derive(Default)]
pub struct Tray {
    items: BTreeMap<StatusNotifierId, Item>,
}

impl Tray {
    pub fn remove(&mut self, id: &StatusNotifierId) {
        self.items.remove(id);
    }
    pub fn update(&mut self, item: StatusNotifierItem) {
        let icon = match icon(&item) {
            Ok(icon) => icon,
            Err(error) => {
                log::warn!("Tray icon: {error:#}");
                None
            }
        };
        let menu = item
            .menu
            .filter(|path| path != "/")
            .map(|path| MenuId::new(item.id.service(), path));
        self.items
            .insert(item.id, Item { icon, menu });
    }

    pub fn view(&self) -> Element<'_, TrayMessage> {
        row(self.items.values().map(|item| {
            let icon: Element<'_, TrayMessage> = match &item.icon {
                Some(Icon::Svg(handle)) => iced::widget::svg(handle.clone())
                    .width(18)
                    .height(18)
                    .content_fit(ContentFit::Contain)
                    .into(),
                Some(Icon::Image(handle)) => iced::widget::image(handle.clone())
                    .width(18)
                    .height(18)
                    .content_fit(ContentFit::Contain)
                    .into(),
                None => text("•").into(),
            };
            let button = button(container(icon).center_y(Fill))
                .width(18)
                .height(Fill)
                .padding(0)
                .style(|theme, _| button::Style {
                    text_color: theme.palette().text,
                    ..button::Style::default()
                });
            let button = if let Some(menu) = &item.menu {
                button.on_press(TrayMessage::Open(menu.clone()))
            } else {
                button
            };
            button.into()
        }))
        .spacing(4)
        .into()
    }
}
fn icon(item: &StatusNotifierItem) -> anyhow::Result<Option<Icon>> {
    if let Some(pixmap) = item
        .icon_pixmaps
        .iter()
        .filter(|pixmap| pixmap.width > 0 && pixmap.height > 0)
        .max_by_key(|pixmap| i64::from(pixmap.width) * i64::from(pixmap.height))
    {
        let width = u32::try_from(pixmap.width)?;
        let height = u32::try_from(pixmap.height)?;
        let length = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4));
        anyhow::ensure!(
            length == Some(pixmap.argb.len()),
            "invalid tray pixmap length"
        );
        let mut pixels = pixmap.argb.clone();
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.rotate_left(1);
        }
        return Ok(Some(Icon::Image(iced::widget::image::Handle::from_rgba(
            width, height, pixels,
        ))));
    }
    let Some(name) = item.icon_name.as_deref() else {
        return Ok(None);
    };
    let path = resolve_icon(name, item.icon_theme_path.as_deref());
    Ok(path.map(|path| {
        if path
            .extension()
            .is_some_and(|extension| extension == "svg")
        {
            Icon::Svg(iced::widget::svg::Handle::from_path(path))
        } else {
            Icon::Image(iced::widget::image::Handle::from_path(path))
        }
    }))
}
fn resolve_icon(name: &str, directory: Option<&str>) -> Option<PathBuf> {
    if Path::new(name).is_absolute() {
        return Some(name.into());
    }
    if let Some(directory) = directory {
        for name in [
            name.to_owned(),
            format!("{name}.svg"),
            format!("{name}.png"),
        ] {
            let path = Path::new(directory).join(name);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    let theme = freedesktop_icons::default_theme_gtk();
    let mut lookup = freedesktop_icons::lookup(name)
        .with_size(24)
        .with_cache();
    if let Some(theme) = theme.as_deref() {
        lookup = lookup.with_theme(theme);
    }
    lookup.find()
}
pub struct MenuState {
    pub layout: MenuLayout,
    size: (u32, u32),
    generation: u64,
}
impl MenuState {
    pub fn new(layout: MenuLayout) -> Self {
        let height = layout
            .items()
            .iter()
            .filter(|item| item.visible)
            .map(|item| {
                if item.item_type == MenuItemType::Separator {
                    1_u32
                } else {
                    30
                }
            })
            .fold(8_u32, u32::saturating_add)
            .clamp(38, 702);
        Self {
            layout,
            size: (258, height),
            generation: 0,
        }
    }
    pub fn replace(&mut self, layout: MenuLayout) {
        self.layout = layout;
        self.generation = self.generation.wrapping_add(1);
    }
    pub fn update(&mut self, message: &TrayMessage) -> Option<i32> {
        match message {
            TrayMessage::Resize(width, height) => self.size = (*width, *height),
            TrayMessage::Activate(id) => {
                fn actionable(items: &[MenuItem], id: i32) -> bool {
                    items
                        .iter()
                        .filter(|item| item.visible && item.enabled)
                        .any(|item| {
                            if item.id == id {
                                !item
                                    .children
                                    .iter()
                                    .any(|child| child.visible)
                            } else {
                                actionable(&item.children, id)
                            }
                        })
                }
                return actionable(self.layout.items(), *id).then_some(*id);
            }
            _ => {}
        }
        None
    }
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn view(&self) -> Element<'_, TrayMessage> {
        fn depth(items: &[MenuItem]) -> usize {
            items
                .iter()
                .filter(|item| item.visible && item.enabled)
                .map(|item| depth(&item.children) + 1)
                .max()
                .unwrap_or(0)
        }

        menu_popup::Popup::new(
            menu_items(self.layout.items()),
            depth(self.layout.items()),
            self.generation,
        )
        .into()
    }
}

fn menu_items(
    items: &[MenuItem],
) -> iced_aw::menu::Menu<'_, TrayMessage, iced::Theme, iced::Renderer> {
    use iced_aw::menu::{Item, Menu};

    let mut entries: Vec<_> = items
        .iter()
        .filter(|item| item.visible)
        .map(|item| {
            if item.item_type == MenuItemType::Separator {
                return Item::new(iced::widget::rule::horizontal(1))
                    .close_on_click(false);
            }
            let submenu = item.enabled
                && item
                    .children
                    .iter()
                    .any(|child| child.visible);
            let label = format!(
                "{}{}{}",
                if item.toggle_state == 1 { "✓ " } else { "" },
                item.label.replace('_', ""),
                if submenu { "  ‹" } else { "" }
            );
            let mut entry = button(text(label))
                .width(Fill)
                .height(30)
                .style(|theme: &iced::Theme, status| {
                    if matches!(status, button::Status::Hovered | button::Status::Pressed)
                    {
                        button::Style {
                            background: Some(theme.palette().primary.into()),
                            text_color: Color::BLACK,
                            ..button::Style::default()
                        }
                    } else {
                        ghost_shell_theme::plain(theme, status)
                    }
                });
            if item.enabled {
                entry = entry.on_press(TrayMessage::Activate(item.id));
            }
            if submenu {
                Item::with_menu(entry, menu_items(&item.children))
            } else {
                Item::new(entry)
            }
            .close_on_click(false)
        })
        .collect();

    // iced_aw requires at least one item when laying out an open menu.
    if entries.is_empty() {
        entries.push(Item::new(text("No actions").height(30)));
    }

    Menu::new(entries)
        .width(250)
        .padding(4)
        .offset(8.0)
}
