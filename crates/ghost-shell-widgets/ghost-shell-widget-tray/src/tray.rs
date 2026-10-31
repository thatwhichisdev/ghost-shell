use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use ghost_shell_dbus::{
    MenuId, MenuItem, MenuItemType, MenuLayout, StatusNotifierId, StatusNotifierItem,
};
use iced::{
    ContentFit, Element, Fill,
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

#[derive(Clone)]
enum Icon {
    Symbolic(iced::widget::svg::Handle),
    Svg(iced::widget::svg::Handle),
    Image(iced::widget::image::Handle),
}

pub struct IconRequest {
    snapshot: Arc<StatusNotifierItem>,
    revision: u64,
    generation: u64,
    theme: Option<String>,
    size: u16,
}

#[derive(Clone)]
pub struct IconUpdate {
    id: StatusNotifierId,
    revision: u64,
    generation: u64,
    icon: Result<Option<Icon>, String>,
}

pub fn resolve_icons(requests: Vec<IconRequest>) -> Vec<IconUpdate> {
    requests
        .into_iter()
        .map(|request| IconUpdate {
            icon: icon(&request.snapshot, request.theme.as_deref(), request.size)
                .map_err(|error| error.to_string()),
            id: request.snapshot.id.clone(),
            revision: request.revision,
            generation: request.generation,
        })
        .collect()
}

struct Item {
    revision: u64,
    icon: Option<Icon>,
    menu: Option<MenuId>,
    snapshot: Arc<StatusNotifierItem>,
}

#[derive(Default)]
pub struct Tray {
    theme: Option<String>,
    icon_size: u16,
    generation: u64,
    revision: u64,
    items: BTreeMap<StatusNotifierId, Item>,
}

impl Tray {
    pub fn new(theme: Option<String>, icon_size: f32) -> Self {
        Self {
            theme,
            icon_size: icon_size.ceil().clamp(1.0, u16::MAX as f32) as u16,
            ..Self::default()
        }
    }

    pub fn configure(
        &mut self,
        theme: Option<String>,
        icon_size: f32,
    ) -> Vec<IconRequest> {
        let icon_size = icon_size.ceil().clamp(1.0, u16::MAX as f32) as u16;
        if self.theme == theme && self.icon_size == icon_size {
            return Vec::new();
        }
        self.theme = theme;
        self.icon_size = icon_size;
        self.generation = self.generation.wrapping_add(1);
        self.items
            .values()
            .map(|item| IconRequest {
                snapshot: item.snapshot.clone(),
                revision: item.revision,
                generation: self.generation,
                theme: self.theme.clone(),
                size: self.icon_size,
            })
            .collect()
    }

    pub fn apply_icons(&mut self, updates: Vec<IconUpdate>) -> Vec<String> {
        let mut errors = Vec::new();
        for update in updates {
            if update.generation != self.generation {
                continue;
            }
            if let Some(item) = self.items.get_mut(&update.id) {
                if item.revision != update.revision {
                    continue;
                }
                match update.icon {
                    Ok(icon) => item.icon = icon,
                    Err(error) => errors.push(error),
                }
            }
        }
        errors
    }

    pub fn remove(&mut self, id: &StatusNotifierId) {
        self.items.remove(id);
    }
    pub fn update(&mut self, item: StatusNotifierItem) {
        log::debug!(
            "Tray item {}: identifier={:?}, icon_name={:?}, icon_theme_path={:?}, pixmaps={}",
            item.id,
            item.identifier,
            item.icon_name,
            item.icon_theme_path,
            item.icon_pixmaps.len(),
        );
        let icon = match icon(&item, self.theme.as_deref(), self.icon_size) {
            Ok(icon) => icon,
            Err(error) => {
                log::warn!("Tray icon: {error:#}");
                None
            }
        };
        let menu = item
            .menu
            .clone()
            .filter(|path| path != "/")
            .map(|path| MenuId::new(item.id.service(), path));
        self.revision = self.revision.wrapping_add(1);
        self.items.insert(
            item.id.clone(),
            Item {
                revision: self.revision,
                icon,
                menu,
                snapshot: Arc::new(item),
            },
        );
    }

    pub fn view(&self, style: ghost_shell_theme::bar::Style) -> Element<'_, TrayMessage> {
        if self.items.is_empty() {
            return iced::widget::Space::new()
                .width(0)
                .height(0)
                .into();
        }
        ghost_shell_theme::bar::outer(
            ghost_shell_theme::bar::group(
                row(self
                    .items
                    .values()
                    .enumerate()
                    .map(|(index, item)| {
                        let icon: Element<'_, TrayMessage> = match &item.icon {
                            Some(Icon::Symbolic(handle)) => ghost_shell_theme::bar::svg(
                                handle.clone(),
                                style.widgets.icon_size,
                            ),
                            Some(Icon::Svg(handle)) => iced::widget::svg(handle.clone())
                                .width(style.widgets.icon_size)
                                .height(style.widgets.icon_size)
                                .content_fit(ContentFit::Contain)
                                .style(|_, _| iced::widget::svg::Style { color: None })
                                .into(),
                            Some(Icon::Image(handle)) => {
                                iced::widget::image(handle.clone())
                                    .width(style.widgets.icon_size)
                                    .height(style.widgets.icon_size)
                                    .content_fit(ContentFit::Contain)
                                    .into()
                            }
                            None => container(
                                text("•")
                                    .font(style.font)
                                    .size(style.font_size),
                            )
                            .center(style.widgets.icon_size)
                            .into(),
                        };
                        let content = ghost_shell_theme::bar::group_inner(
                            icon,
                            index == 0,
                            index == self.items.len().saturating_sub(1),
                            style,
                        );
                        if let Some(menu) = &item.menu {
                            button(content)
                                .padding(0)
                                .style(ghost_shell_theme::bar::transparent_button)
                                .on_press(TrayMessage::Open(menu.clone()))
                                .into()
                        } else {
                            content
                        }
                    }))
                .spacing(0),
                style,
            ),
            style,
        )
    }
}
fn icon(
    item: &StatusNotifierItem,
    theme: Option<&str>,
    icon_size: u16,
) -> anyhow::Result<Option<Icon>> {
    if let Some(path) = symbolic_icon(item, theme, icon_size) {
        log::debug!("Tray symbolic icon for {}: {}", item.id, path.display());
        return Ok(Some(Icon::Symbolic(iced::widget::svg::Handle::from_path(
            path,
        ))));
    }
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
    let path = resolve_icon(name, item.icon_theme_path.as_deref(), theme, icon_size);
    path.map(|path| -> anyhow::Result<Icon> {
        if path
            .extension()
            .is_some_and(|extension| extension == "svg")
        {
            Ok(Icon::Svg(iced::widget::svg::Handle::from_path(path)))
        } else {
            let image = image::open(path)?.into_rgba8();
            Ok(Icon::Image(iced::widget::image::Handle::from_rgba(
                image.width(),
                image.height(),
                image.into_raw(),
            )))
        }
    })
    .transpose()
}
fn symbolic_icon(
    item: &StatusNotifierItem,
    configured_theme: Option<&str>,
    icon_size: u16,
) -> Option<PathBuf> {
    let detected_theme = configured_theme
        .is_none()
        .then(freedesktop_icons::default_theme_gtk)
        .flatten();
    let themes = if let Some(theme) = configured_theme {
        vec![theme]
    } else {
        vec![
            detected_theme
                .as_deref()
                .unwrap_or("hicolor"),
            "Papirus-Dark",
            "Papirus",
            "Adwaita",
        ]
    };
    for name in [item.icon_name.as_deref(), item.identifier.as_deref()]
        .into_iter()
        .flatten()
    {
        let Some(name) = Path::new(name)
            .file_name()
            .and_then(|name| name.to_str())
        else {
            continue;
        };
        let name = name
            .strip_suffix(".svg")
            .or_else(|| name.strip_suffix(".png"))
            .unwrap_or(name);
        if name.is_empty() {
            continue;
        }
        let symbolic = if name.ends_with("-symbolic") {
            name.to_owned()
        } else {
            format!("{name}-symbolic")
        };
        for theme in &themes {
            if let Some(path) = freedesktop_icons::lookup(&symbolic)
                .with_theme(theme)
                .with_size(icon_size)
                .force_svg()
                .with_cache()
                .find()
                .filter(|path| {
                    path.extension()
                        .is_some_and(|extension| extension == "svg")
                })
            {
                return Some(path);
            }
        }
    }
    None
}

fn resolve_icon(
    name: &str,
    directory: Option<&str>,
    theme: Option<&str>,
    icon_size: u16,
) -> Option<PathBuf> {
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
    let detected_theme = theme
        .is_none()
        .then(freedesktop_icons::default_theme_gtk)
        .flatten();
    let theme = theme.or(detected_theme.as_deref());
    let mut lookup = freedesktop_icons::lookup(name)
        .with_size(icon_size)
        .with_cache();
    if let Some(theme) = theme {
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
                    15_u32
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

    pub fn view(&self, font: iced::Font, font_size: f32) -> Element<'_, TrayMessage> {
        fn depth(items: &[MenuItem]) -> usize {
            items
                .iter()
                .filter(|item| item.visible && item.enabled)
                .map(|item| depth(&item.children) + 1)
                .max()
                .unwrap_or(0)
        }

        menu_popup::Popup::new(
            menu_items(self.layout.items(), font, font_size),
            depth(self.layout.items()),
            self.generation,
        )
        .into()
    }
}

fn menu_items(
    items: &[MenuItem],
    font: iced::Font,
    font_size: f32,
) -> iced_aw::menu::Menu<'_, TrayMessage, iced::Theme, iced::Renderer> {
    use iced_aw::menu::{Item, Menu};

    let mut entries: Vec<_> = items
        .iter()
        .filter(|item| item.visible)
        .map(|item| {
            if item.item_type == MenuItemType::Separator {
                return Item::new(
                    container(iced::widget::rule::horizontal(1))
                        .width(Fill)
                        .padding([0, 9])
                        .center_y(15),
                )
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
            let mut entry = button(
                row![
                    iced::widget::Space::new()
                        .width(0)
                        .height(22),
                    ghost_shell_theme::label(label, font, font_size).width(Fill),
                ]
                .align_y(iced::Center),
            )
            .width(Fill)
            .padding([0, 5])
            .style(ghost_shell_theme::menu_item);
            if item.enabled {
                entry = entry.on_press(TrayMessage::Activate(item.id));
            }
            let entry = container(entry).width(Fill).padding(4);
            if submenu {
                Item::with_menu(entry, menu_items(&item.children, font, font_size))
            } else {
                Item::new(entry)
            }
            .close_on_click(false)
        })
        .collect();

    // iced_aw requires at least one item when laying out an open menu.
    if entries.is_empty() {
        entries.push(Item::new(
            ghost_shell_theme::label("No actions", font, font_size).height(30),
        ));
    }

    Menu::new(entries)
        .width(250)
        .padding(4)
        .offset(8.0)
}
