use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use ghost_shell_dbus::{
    MenuId, MenuItem, MenuItemType, MenuLayout, StatusNotifierId, StatusNotifierItem,
};
use iced::{
    Border, Color, ContentFit, Element, Fill,
    widget::{button, column, container, row, text},
};
#[derive(Clone, Debug)]
pub enum TrayMessage {
    Open(MenuId),
    Hover(usize, i32),
    Move(bool),
    Back,
    Enter,
    Activate(usize, i32),
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
    path: Vec<i32>,
    selected: Option<i32>,
}
impl MenuState {
    pub fn new(layout: MenuLayout) -> Self {
        Self {
            layout,
            path: Vec::new(),
            selected: None,
        }
    }
    pub fn replace(&mut self, layout: MenuLayout) {
        self.layout = layout;
        self.path.clear();
        self.selected = None;
    }
    fn items_at(&self, depth: usize) -> &[MenuItem] {
        let mut items = self.layout.items();
        for id in self.path.iter().take(depth) {
            if let Some(item) = items.iter().find(|item| item.id == *id) {
                items = &item.children;
            } else {
                break;
            }
        }
        items
    }
    pub fn update(&mut self, message: &TrayMessage) -> Option<i32> {
        match message {
            TrayMessage::Hover(depth, id) => {
                let submenu = self
                    .items_at(*depth)
                    .iter()
                    .find(|item| item.id == *id)
                    .is_some_and(|item| item.enabled && !item.children.is_empty());
                self.path.truncate(*depth);
                self.selected = Some(*id);
                if submenu {
                    self.path.push(*id);
                }
            }
            TrayMessage::Move(forward) => {
                let items: Vec<_> = self
                    .items_at(self.path.len())
                    .iter()
                    .filter(|item| {
                        item.visible
                            && item.enabled
                            && item.item_type != MenuItemType::Separator
                    })
                    .collect();
                let position = self
                    .selected
                    .and_then(|id| items.iter().position(|item| item.id == id));
                self.selected =
                    ghost_shell_theme::selection(position, items.len(), *forward)
                        .and_then(|index| items.get(index))
                        .map(|item| item.id);
            }
            TrayMessage::Back => {
                self.selected = self.path.pop();
            }
            TrayMessage::Enter => {
                if let Some(id) = self.selected {
                    return self.activate(self.path.len(), id);
                }
            }
            TrayMessage::Activate(depth, id) => return self.activate(*depth, *id),
            _ => {}
        }
        None
    }
    fn activate(&mut self, depth: usize, id: i32) -> Option<i32> {
        self.path.truncate(depth);
        let item = self
            .items_at(self.path.len())
            .iter()
            .find(|item| item.id == id)?;
        if !item.enabled {
            return None;
        }
        if item.children.is_empty() {
            Some(id)
        } else {
            self.path.push(id);
            self.selected = None;
            None
        }
    }
    pub fn size(&self) -> (u32, u32) {
        fn measure(items: &[MenuItem]) -> (usize, usize) {
            items
                .iter()
                .fold((1, items.len()), |(depth, rows), item| {
                    let (child_depth, child_rows) = measure(&item.children);
                    (
                        depth.max(if item.children.is_empty() {
                            1
                        } else {
                            child_depth + 1
                        }),
                        rows.max(child_rows),
                    )
                })
        }
        let (depth, rows) = measure(self.layout.items());
        (
            (depth.min(5) * 250 + 2) as u32,
            (rows * 30 + 14).clamp(62, 702) as u32,
        )
    }
    pub fn view(&self) -> Element<'_, TrayMessage> {
        let columns = (0..=self.path.len()).map(|depth| {
            let items = self
                .items_at(depth)
                .iter()
                .filter(|item| item.visible)
                .map(move |item| {
                    if item.item_type == MenuItemType::Separator {
                        return iced::widget::rule::horizontal(1).into();
                    }
                    let label = format!(
                        "{}{}{}",
                        if item.toggle_state == 1 { "✓ " } else { "" },
                        item.label.replace('_', ""),
                        if item.children.is_empty() {
                            ""
                        } else {
                            "  ›"
                        }
                    );
                    let button = button(text(label))
                        .width(Fill)
                        .height(30)
                        .style(move |theme: &iced::Theme, status| {
                            if item.enabled && self.selected == Some(item.id) {
                                button::Style {
                                    background: Some(theme.palette().primary.into()),
                                    text_color: Color::BLACK,
                                    ..button::Style::default()
                                }
                            } else {
                                ghost_shell_theme::plain(theme, status)
                            }
                        });
                    let button = if item.enabled {
                        button.on_press(TrayMessage::Activate(depth, item.id))
                    } else {
                        button
                    };
                    iced::widget::mouse_area(button)
                        .on_enter(TrayMessage::Hover(depth, item.id))
                        .into()
                });
            container(iced::widget::scrollable(column(items)))
                .width(250)
                .height(Fill)
                .padding(4)
                .into()
        });
        container(row(columns).height(Fill))
            .height(Fill)
            .padding(1)
            .style(|theme| container::Style {
                border: Border {
                    color: Color::WHITE,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..ghost_shell_theme::surface(theme, 1.0)
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: i32, children: Vec<MenuItem>) -> MenuItem {
        MenuItem {
            id,
            item_type: MenuItemType::Standard,
            label: id.to_string(),
            enabled: true,
            visible: true,
            icon_name: None,
            icon_data: vec![],
            shortcut: vec![],
            toggle_type: None,
            toggle_state: 0,
            children_display: None,
            children,
        }
    }
    #[test]
    fn hovering_submenu_does_not_break_clicks_in_parent_column() {
        let mut state = MenuState::new(MenuLayout {
            id: MenuId::new("test", "/menu"),
            revision: 1,
            root: item(0, vec![item(1, vec![item(2, vec![])]), item(3, vec![])]),
        });
        assert_eq!(state.update(&TrayMessage::Hover(0, 1)), None);
        assert_eq!(state.path, vec![1]);
        assert_eq!(state.update(&TrayMessage::Activate(1, 2)), Some(2));
        assert_eq!(state.update(&TrayMessage::Activate(0, 3)), Some(3));
        assert!(state.path.is_empty());
        state.update(&TrayMessage::Hover(0, 1));
        assert_eq!(state.update(&TrayMessage::Back), None);
        assert_eq!(state.selected, Some(1));
        assert_eq!(state.update(&TrayMessage::Enter), None);
        assert_eq!(state.path, vec![1]);
    }
}
