use anyhow::{Result, anyhow};
use ghost_shell_dbus::StatusNotifierItem;

use crate::{icon::TrayIcon, menu::TrayMenu};

pub(crate) struct TrayItem {
    pub(crate) icon: TrayIcon,
    pub(crate) menu: Option<TrayMenu>,
}

impl TrayItem {
    pub(crate) fn new(item: &StatusNotifierItem, menu: Option<TrayMenu>) -> Result<Self> {
        Ok(Self {
            icon: icon(item)?,
            menu,
        })
    }

    pub(crate) fn update(&mut self, item: &StatusNotifierItem) -> Result<()> {
        self.icon = icon(item)?;

        Ok(())
    }
}

impl TryFrom<&StatusNotifierItem> for TrayItem {
    type Error = anyhow::Error;

    fn try_from(item: &StatusNotifierItem) -> Result<Self> {
        Self::new(item, None)
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
