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

        return TrayIcon::from_path(path);
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
