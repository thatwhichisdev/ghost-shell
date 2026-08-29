mod client;

use std::fmt;

use anyhow::{Result, bail};

pub(crate) use client::StatusNotifierItemClient;

const DEFAULT_ITEM_PATH: &str = "/StatusNotifierItem";

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StatusNotifierId {
    service: String,
    object_path: String,
}

impl StatusNotifierId {
    pub(crate) fn from_registered_item(item: &str) -> Result<Self> {
        let (service, object_path) = match item.find('/') {
            Some(index) if index > 0 => (&item[..index], &item[index..]),
            Some(_) => {
                bail!("status notifier item is missing a service name");
            }
            None => (item, DEFAULT_ITEM_PATH),
        };

        if service.is_empty() {
            bail!("status notifier item is missing a service name");
        }

        Ok(Self {
            service: service.to_owned(),
            object_path: object_path.to_owned(),
        })
    }

    #[must_use]
    pub fn service(&self) -> &str {
        &self.service
    }

    #[must_use]
    pub fn object_path(&self) -> &str {
        &self.object_path
    }
}

impl fmt::Display for StatusNotifierId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", self.service, self.object_path,)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IconPixmap {
    pub width: i32,
    pub height: i32,
    pub argb: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolTip {
    pub icon_name: String,
    pub icon_pixmaps: Vec<IconPixmap>,
    pub title: String,
    pub description: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusNotifierItem {
    pub id: StatusNotifierId,

    pub category: Option<String>,
    pub identifier: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub window_id: Option<u32>,

    pub icon_name: Option<String>,
    pub icon_pixmaps: Vec<IconPixmap>,

    pub overlay_icon_name: Option<String>,
    pub overlay_icon_pixmaps: Vec<IconPixmap>,

    pub attention_icon_name: Option<String>,
    pub attention_icon_pixmaps: Vec<IconPixmap>,
    pub attention_movie_name: Option<String>,

    pub tool_tip: Option<ToolTip>,
    pub item_is_menu: Option<bool>,
    pub menu: Option<String>,

    // KDE extension used by real-world implementations.
    pub icon_theme_path: Option<String>,
}

impl StatusNotifierItem {
    #[must_use]
    pub(crate) fn empty(id: StatusNotifierId) -> Self {
        Self {
            id,
            category: None,
            identifier: None,
            title: None,
            status: None,
            window_id: None,
            icon_name: None,
            icon_pixmaps: Vec::new(),
            overlay_icon_name: None,
            overlay_icon_pixmaps: Vec::new(),
            attention_icon_name: None,
            attention_icon_pixmaps: Vec::new(),
            attention_movie_name: None,
            tool_tip: None,
            item_is_menu: None,
            menu: None,
            icon_theme_path: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum StatusNotifierItemEvent {
    Updated(StatusNotifierItem),
}

#[cfg(test)]
mod tests {
    use super::StatusNotifierId;

    #[test]
    fn parses_registered_item_with_default_path() {
        let id = StatusNotifierId::from_registered_item("org.example.Item")
            .expect("valid status notifier item");

        assert_eq!(id.service(), "org.example.Item");
        assert_eq!(id.object_path(), "/StatusNotifierItem",);
    }

    #[test]
    fn parses_registered_item_with_explicit_path() {
        let id = StatusNotifierId::from_registered_item(
            ":1.42/org/example/StatusNotifierItem",
        )
        .expect("valid status notifier item");

        assert_eq!(id.service(), ":1.42");
        assert_eq!(id.object_path(), "/org/example/StatusNotifierItem",);
    }
}
