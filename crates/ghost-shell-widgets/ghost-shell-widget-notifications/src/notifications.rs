use std::{collections::BTreeMap, path::PathBuf};

use ghost_shell_dbus::Notification;
use iced::{
    Element, Fill,
    widget::{button, column, container, mouse_area, row, scrollable, sensor, text},
};

#[derive(Clone, Debug)]
pub enum Message {
    Open,
    Bounds(iced::Rectangle),
    ToggleMuted,
    Read(u32),
    Resize(u32, u32),
}

#[derive(Clone, Debug)]
pub struct IconUpdate {
    pub id: u32,
    pub revision: u64,
    pub path: Option<PathBuf>,
}

struct Entry {
    notification: Notification,
    icon: Option<PathBuf>,
}

#[derive(Default)]
pub struct Notifications {
    entries: BTreeMap<u32, Entry>,
    pub muted: bool,
    pub error: Option<String>,
    pub ready: bool,
}

impl Notifications {
    pub fn update(&mut self, notification: Notification) {
        self.entries.insert(
            notification.id,
            Entry {
                notification,
                icon: None,
            },
        );
    }

    pub fn close(&mut self, id: u32) {
        self.entries.remove(&id);
    }

    pub fn apply_icon(&mut self, update: IconUpdate) {
        if let Some(entry) = self.entries.get_mut(&update.id)
            && entry.notification.revision == update.revision
        {
            entry.icon = update.path;
        }
    }

    pub fn snapshots(&self) -> Vec<Notification> {
        self.entries
            .values()
            .map(|entry| entry.notification.clone())
            .collect()
    }

    fn icon_name(&self) -> &'static str {
        if self.muted {
            "icons/bell-simple-slash-fill.svg"
        } else if self.entries.is_empty() {
            "icons/bell-simple-fill.svg"
        } else {
            "icons/bell-simple-ringing-fill.svg"
        }
    }

    pub fn view(&self, style: ghost_shell_theme::bar::Style) -> Element<'_, Message> {
        let icon = self.icon_name();
        ghost_shell_theme::bar::outer(
            ghost_shell_theme::bar::anchor(
                mouse_area(
                    button(ghost_shell_theme::bar::inner(
                        ghost_shell_theme::bar::icon(icon, style.widgets.icon_size),
                        style,
                    ))
                    .padding(0)
                    .style(ghost_shell_theme::bar::transparent_button)
                    .on_press(Message::Open),
                )
                .on_right_press(Message::ToggleMuted),
                style,
                "notifications".into(),
                Message::Bounds,
            ),
            style,
        )
    }

    pub fn popup(
        &self,
        font: iced::Font,
        font_size: f32,
        opacity: f32,
    ) -> Element<'_, Message> {
        let label = |value| ghost_shell_theme::label(value, font, font_size);
        let mut items = column![label(if self.muted {
            "Notifications · muted"
        } else {
            "Notifications"
        })]
        .spacing(8);
        if let Some(error) = &self.error {
            items = items.push(label(error.as_str()).style(text::danger));
        }
        if !self.ready {
            items = items.push(label("Notification service unavailable."));
        } else if self.entries.is_empty() {
            items = items.push(label("No unread notifications."));
        }
        // Revisions also order replacements as newly received notifications.
        let mut entries: Vec<_> = self.entries.values().collect();
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.notification.revision));
        for entry in entries {
            let notification = &entry.notification;
            let icon: Element<'_, Message> = match &entry.icon {
                Some(path)
                    if path
                        .extension()
                        .is_some_and(|extension| extension == "svg") =>
                {
                    iced::widget::svg(iced::widget::svg::Handle::from_path(path))
                        .width(24)
                        .height(24)
                        .into()
                }
                Some(path) => {
                    iced::widget::image(iced::widget::image::Handle::from_path(path))
                        .width(24)
                        .height(24)
                        .into()
                }
                None => ghost_shell_theme::bar::icon("icons/bell-simple-fill.svg", 24.0),
            };
            let mut content = column![
                label(if notification.app_name.is_empty() {
                    "Application"
                } else {
                    &notification.app_name
                }),
                label(&notification.summary)
            ]
            .spacing(4)
            .width(Fill);
            if !notification.body.is_empty() {
                content = content.push(label(&notification.body));
            }
            items = items.push(
                container(
                    column![
                        row![icon, content].spacing(8),
                        button(label("Mark as read"))
                            .style(ghost_shell_theme::menu_item)
                            .on_press_maybe(
                                self.ready
                                    .then_some(Message::Read(notification.id))
                            ),
                    ]
                    .spacing(6),
                )
                .padding(8)
                .width(Fill),
            );
            items = items.push(iced::widget::rule::horizontal(1));
        }
        let measured = sensor(items)
            .on_show(resize)
            .on_resize(resize);
        container(scrollable(measured).style(ghost_shell_theme::search_scrollable))
            .padding(8)
            .width(Fill)
            .height(Fill)
            .style(move |theme| {
                ghost_shell_theme::menu_surface_with_opacity(theme, opacity)
            })
            .into()
    }
}

fn resize(size: iced::Size) -> Message {
    Message::Resize(
        400,
        (size.height.ceil() as u32)
            .saturating_add(16)
            .clamp(48, 600),
    )
}

pub fn resolve_icon(notification: Notification, theme: Option<String>) -> IconUpdate {
    let theme = theme.or_else(freedesktop_icons::default_theme_gtk);
    let mut name = notification.app_icon.clone();
    if name.is_empty()
        && let Some(desktop) = &notification.desktop_entry
    {
        let locales = freedesktop_desktop_entry::get_languages_from_env();
        if let Some(entry) = freedesktop_desktop_entry::desktop_entries(&locales)
            .into_iter()
            .find(|entry| {
                entry
                    .path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    == Some(desktop.as_str())
            })
        {
            name = entry.icon().unwrap_or_default().to_owned();
        }
    }
    let path = if name.starts_with('/') {
        let path = PathBuf::from(name);
        path.is_file().then_some(path)
    } else if name.is_empty() {
        None
    } else {
        let mut lookup = freedesktop_icons::lookup(&name)
            .with_size(24)
            .with_cache();
        if let Some(theme) = &theme {
            lookup = lookup.with_theme(theme);
        }
        lookup.find()
    };
    IconUpdate {
        id: notification.id,
        revision: notification.revision,
        path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn muting_retains_unread_items_and_stale_icons_cannot_restore_them() {
        let mut state = Notifications::default();
        assert_eq!(state.icon_name(), "icons/bell-simple-fill.svg");
        let mut notification = Notification {
            id: 1,
            revision: 1,
            app_name: "Test".into(),
            app_icon: String::new(),
            desktop_entry: None,
            summary: "First".into(),
            body: String::new(),
        };
        state.update(notification.clone());
        assert_eq!(state.icon_name(), "icons/bell-simple-ringing-fill.svg");
        state.muted = true;
        assert_eq!(state.icon_name(), "icons/bell-simple-slash-fill.svg");
        assert_eq!(state.entries.len(), 1);
        notification.revision = 2;
        notification.summary = "Replacement".into();
        state.update(notification);
        state.apply_icon(IconUpdate {
            id: 1,
            revision: 1,
            path: Some("stale.svg".into()),
        });
        assert!(
            state
                .entries
                .get(&1)
                .is_some_and(|entry| entry.icon.is_none())
        );
        state.muted = false;
        assert_eq!(state.icon_name(), "icons/bell-simple-ringing-fill.svg");
        state.close(1);
        state.apply_icon(IconUpdate {
            id: 1,
            revision: 2,
            path: Some("late.svg".into()),
        });
        assert!(state.entries.is_empty());
        assert_eq!(state.icon_name(), "icons/bell-simple-fill.svg");
    }
}
