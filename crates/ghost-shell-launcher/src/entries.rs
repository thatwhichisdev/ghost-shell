use std::path::PathBuf;

use gpui::{App, Global};

#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub name: String,
    pub command: Vec<String>,
    pub icon: Option<PathBuf>,
    pub description: Option<String>,
    pub terminal: bool,
}

impl AsRef<str> for DesktopEntry {
    #[inline]
    fn as_ref(&self) -> &str {
        self.name.as_str()
    }
}

#[derive(Clone)]
pub struct DesktopEntries {
    pub items: Vec<DesktopEntry>,
}

impl Global for DesktopEntries {}

pub fn init(cx: &mut App) {
    let locales = freedesktop_desktop_entry::get_languages_from_env();
    let theme = freedesktop_icons::default_theme_gtk();

    let apps = freedesktop_desktop_entry::desktop_entries(&locales)
        .into_iter()
        .filter(|entry| !entry.no_display())
        .map(|entry| {
            let name = entry.name(&locales).unwrap().into_owned();
            let command = entry
                .parse_exec_with_uris(&[], &locales)
                .unwrap();
            let description = entry
                .comment(&locales)
                .map(|d| d.to_string());
            let icon = entry.icon().and_then(|i| {
                let mut icon_builder = freedesktop_icons::lookup(i)
                    .with_size(64)
                    .with_scale(1)
                    .force_svg()
                    .with_cache();

                if let Some(theme) = theme.as_deref() {
                    icon_builder = icon_builder.with_theme(theme);
                }

                icon_builder.find()
            });
            let terminal = entry.terminal();

            DesktopEntry {
                name,
                command,
                icon,
                description,
                terminal,
            }
        })
        .collect();

    let desktop_entries = DesktopEntries { items: apps };

    cx.set_global(desktop_entries);
}
