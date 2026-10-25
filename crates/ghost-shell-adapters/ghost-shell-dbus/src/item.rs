use std::fmt;

use anyhow::{Result, bail};

const DEFAULT_ITEM_PATH: &str = "/StatusNotifierItem";

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StatusNotifierId {
    service: String,
    object_path: String,
}

impl StatusNotifierId {
    pub fn from_registration(registration: &str) -> Result<Self> {
        let (service, object_path) = match registration.find('/') {
            Some(index) if index > 0 => (&registration[..index], &registration[index..]),

            Some(_) => {
                bail!(
                    "status notifier registration \
                         is missing a service name"
                );
            }

            None => (registration, DEFAULT_ITEM_PATH),
        };

        if service.is_empty() {
            bail!(
                "status notifier registration \
                 is missing a service name"
            );
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

    /// KDE extension used by real-world StatusNotifierItem
    /// implementations.
    pub icon_theme_path: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ItemEvent {
    Updated(StatusNotifierItem),
}
