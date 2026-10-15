use std::path::PathBuf;

use ghost_shell_gpui::{App, Global};

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
        .filter_map(|entry| desktop_entry(&entry, &locales, theme.as_deref()))
        .collect();

    let desktop_entries = DesktopEntries { items: apps };

    cx.set_global(desktop_entries);
}

fn desktop_entry(
    entry: &freedesktop_desktop_entry::DesktopEntry,
    locales: &[String],
    theme: Option<&str>,
) -> Option<DesktopEntry> {
    if entry.no_display() {
        return None;
    }

    let Some(name) = entry.name(locales) else {
        log::warn!(
            "Skipping desktop entry {}: missing Name",
            entry.path.display()
        );
        return None;
    };

    let command = match entry.parse_exec_with_uris(&[], locales) {
        Ok(command) => command,
        Err(error) => {
            log::warn!("Skipping desktop entry {}: {error}", entry.path.display());
            return None;
        }
    };

    let description = entry
        .comment(locales)
        .map(|description| description.to_string());

    let icon = entry.icon().and_then(|icon| {
        let mut icon_builder = freedesktop_icons::lookup(icon)
            .with_size(64)
            .with_scale(1)
            .force_svg()
            .with_cache();

        if let Some(theme) = theme {
            icon_builder = icon_builder.with_theme(theme);
        }

        icon_builder.find()
    });

    Some(DesktopEntry {
        name: name.into_owned(),
        command,
        icon,
        description,
        terminal: entry.terminal(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_continues_after_rejected_entries() -> anyhow::Result<()> {
        let locales = vec!["en".to_string()];
        let inputs = [
            "Name=Unmatched quote\nExec=\"/bin/example",
            "Name=Quoted executable\nExec=\"/bin/example\" --flag",
            "Name=Missing command",
            "Name=Empty command\nExec=",
            "Exec=/bin/example",
            "Name=Hidden\nExec=/bin/example\nNoDisplay=true",
            "Name=Working\nExec=/bin/example --flag %U\nComment=Example application\nTerminal=true",
        ];
        let entries = inputs
            .iter()
            .map(|input| {
                freedesktop_desktop_entry::DesktopEntry::from_str(
                    "/applications/example.desktop",
                    &format!("[Desktop Entry]\nType=Application\n{input}\n"),
                    Some(&locales),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let applications = entries
            .iter()
            .filter_map(|entry| desktop_entry(entry, &locales, None))
            .collect::<Vec<_>>();

        assert_eq!(applications.len(), 1);
        let application = applications
            .first()
            .ok_or_else(|| anyhow::anyhow!("missing valid application"))?;
        assert_eq!(application.name, "Working");
        assert_eq!(application.command, ["/bin/example", "--flag"]);
        assert_eq!(
            application.description.as_deref(),
            Some("Example application")
        );
        assert!(application.terminal);
        Ok(())
    }
}
