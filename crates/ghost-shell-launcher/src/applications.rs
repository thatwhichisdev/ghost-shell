use std::path::PathBuf;

use gpui::Global;

#[derive(Debug, Clone)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub icon: Option<PathBuf>,
    pub desc: Option<String>,
}

#[derive(Clone)]
pub struct Applications {
    pub items: Vec<Application>,
}

impl Global for Applications {}

pub fn load() -> Applications {
    let locales = freedesktop_desktop_entry::get_languages_from_env();
    let apps = freedesktop_desktop_entry::desktop_entries(&locales)
        .into_iter()
        .map(|entry| {
            let id = entry.id().to_string();
            let name = entry.name(&locales).unwrap().into_owned();
            let exec = entry.exec().unwrap().to_string();
            let desc = entry.comment(&locales).map(|d| d.to_string());

            let icon = entry
                .icon()
                .and_then(|i| freedesktop_icons::lookup(i).with_cache().find());

            Application {
                id,
                name,
                exec,
                icon,
                desc,
            }
        })
        .collect();

    Applications { items: apps }
}
