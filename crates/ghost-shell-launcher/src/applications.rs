use std::path::PathBuf;

use gpui::Global;

#[derive(Debug, Clone)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub command: Vec<String>,
    pub icon: Option<PathBuf>,
    pub description: Option<String>,
}

impl AsRef<str> for Application {
    #[inline]
    fn as_ref(&self) -> &str {
        self.name.as_str()
    }
}

#[derive(Clone)]
pub struct Applications {
    pub items: Vec<Application>,
}

impl Global for Applications {}

pub fn load() -> Applications {
    let locales = freedesktop_desktop_entry::get_languages_from_env();
    let theme = freedesktop_icons::default_theme_gtk();

    let apps = freedesktop_desktop_entry::desktop_entries(&locales)
        .into_iter()
        .map(|entry| {
            let id = entry.id().to_string();
            let name = entry.name(&locales).unwrap().into_owned();
            let command = entry.parse_exec_with_uris(&[], &locales).unwrap();
            let description = entry.comment(&locales).map(|d| d.to_string());
            let icon = entry.icon().and_then(|i| {
                let mut icon_builder = freedesktop_icons::lookup(i)
                    .with_size(40)
                    .with_scale(2)
                    .with_cache();

                if let Some(theme) = theme.as_deref() {
                    icon_builder = icon_builder.with_theme(theme);
                }

                icon_builder.find()
            });

            Application {
                id,
                name,
                command,
                icon,
                description,
            }
        })
        .collect();

    Applications { items: apps }
}
