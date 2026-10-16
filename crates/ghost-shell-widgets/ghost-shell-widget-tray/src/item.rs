use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use ghost_shell_dbus::{MenuId, StatusNotifierItem};

use crate::icon::TrayIcon;

pub(crate) struct TrayItem {
    pub(crate) icon: TrayIcon,
    pub(crate) menu: Option<MenuId>,
}

impl TrayItem {
    pub(crate) fn new(item: &StatusNotifierItem) -> Result<Self> {
        Ok(Self {
            icon: icon(item)?,
            menu: menu_id(item),
        })
    }

    pub(crate) fn update(&mut self, item: &StatusNotifierItem) -> Result<()> {
        self.icon = icon(item)?;
        self.menu = menu_id(item);

        Ok(())
    }
}

impl TryFrom<&StatusNotifierItem> for TrayItem {
    type Error = anyhow::Error;

    fn try_from(item: &StatusNotifierItem) -> Result<Self> {
        Self::new(item)
    }
}

fn icon(item: &StatusNotifierItem) -> Result<TrayIcon> {
    if item.icon_pixmaps.is_empty() {
        let path = item
            .icon_name
            .clone()
            .ok_or_else(|| anyhow!("status notifier item has no icon"))?;

        return TrayIcon::from_path(resolve_icon(
            &path,
            item.icon_theme_path.as_deref(),
        )?);
    }

    let pixmap = item
        .icon_pixmaps
        .iter()
        .filter(|pixmap| pixmap.width > 0 && pixmap.height > 0)
        .max_by_key(|pixmap| i64::from(pixmap.width) * i64::from(pixmap.height))
        .ok_or_else(|| anyhow!("status notifier item has no usable icon pixmap"))?;

    TrayIcon::from_pixmap(pixmap)
}

fn menu_id(item: &StatusNotifierItem) -> Option<MenuId> {
    item.menu
        .as_deref()
        .filter(|path| *path != "/")
        .map(|path| MenuId::new(item.id.service(), path))
}

fn resolve_icon(name: &str, theme_path: Option<&str>) -> Result<PathBuf> {
    if Path::new(name).is_absolute() {
        return Ok(PathBuf::from(name));
    }
    if let Some(directory) = theme_path.filter(|directory| !directory.is_empty()) {
        for filename in [
            name.to_owned(),
            format!("{name}.svg"),
            format!("{name}.png"),
        ] {
            let path = Path::new(directory).join(filename);
            if path.is_file() {
                return Ok(path);
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
    lookup
        .find()
        .ok_or_else(|| anyhow!("tray icon {name:?} was not found in the icon theme"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_and_loads_named_svg_from_application_directory() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory
            .path()
            .join("gpgui-disconnected-symbolic.svg");
        std::fs::write(&path, br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><circle cx="12" cy="12" r="10"/></svg>"#)?;
        let resolved =
            resolve_icon("gpgui-disconnected-symbolic", directory.path().to_str())?;
        assert_eq!(resolved, path);
        TrayIcon::from_path(&resolved)?;
        let absolute = path
            .to_str()
            .ok_or_else(|| anyhow!("non-UTF-8 test path"))?;
        assert_eq!(resolve_icon(absolute, None)?, path);
        directory.close()?;
        Ok(())
    }
}
