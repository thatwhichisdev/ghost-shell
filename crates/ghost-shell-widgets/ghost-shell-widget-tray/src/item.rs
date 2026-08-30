use anyhow::{Result, anyhow};
use ghost_shell_dbus::StatusNotifierItem;

use crate::icon::TrayIcon;

pub(crate) struct TrayItem {
    pub(crate) icon: TrayIcon,
}

impl TryFrom<&StatusNotifierItem> for TrayItem {
    type Error = anyhow::Error;

    fn try_from(value: &StatusNotifierItem) -> Result<Self> {
        let icon = if value.icon_pixmaps.is_empty() {
            let path = value
                .icon_name
                .clone()
                .ok_or_else(|| anyhow!("status notifier item has no icon"))?;

            TrayIcon::from_path(path)?
        } else {
            let pixmap = value
                .icon_pixmaps
                .iter()
                .filter(|pixmap| pixmap.width > 0 && pixmap.height > 0)
                .max_by_key(|pixmap| i64::from(pixmap.width) * i64::from(pixmap.height))
                .ok_or_else(|| {
                    anyhow!("status notifier item has no usable icon pixmap")
                })?;

            TrayIcon::from_pixmap(pixmap)?
        };

        Ok(Self { icon })
    }
}
