use std::{collections::HashMap, fmt};

use anyhow::{Result, bail};
use zbus::{
    fdo::PropertiesProxy,
    names::InterfaceName,
    proxy,
    zvariant::{OwnedObjectPath, OwnedValue},
};

pub(crate) const STATUS_NOTIFIER_ITEM_INTERFACE: &str = "org.kde.StatusNotifierItem";
const DEFAULT_ITEM_PATH: &str = "/StatusNotifierItem";

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StatusNotifierId {
    service: String,
    object_path: String,
}

impl StatusNotifierId {
    #[must_use]
    pub fn service(&self) -> &str {
        &self.service
    }

    #[must_use]
    pub fn object_path(&self) -> &str {
        &self.object_path
    }

    pub(crate) fn from_registered_item(item: &str) -> Result<Self> {
        let (service, object_path) = match item.find('/') {
            Some(index) if index > 0 => (&item[..index], &item[index..]),
            Some(_) => bail!("status notifier item is missing a service name"),
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
}

impl fmt::Display for StatusNotifierId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", self.service, self.object_path)
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
    pub icon_theme_path: Option<String>,
}

impl StatusNotifierItem {
    #[must_use]
    pub fn empty(id: StatusNotifierId) -> Self {
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

pub(crate) async fn fetch_status_notifier_item(
    connection: &zbus::Connection,
    id: StatusNotifierId,
) -> Result<StatusNotifierItem> {
    let proxy = PropertiesProxy::builder(connection)
        .destination(id.service())?
        .path(id.object_path())?
        .build()
        .await?;
    let interface = InterfaceName::try_from(STATUS_NOTIFIER_ITEM_INTERFACE)?;
    let mut properties = proxy.get_all(interface).await?;

    let tool_tip = take_property::<RawToolTip>(&mut properties, "ToolTip").map(
        |(icon_name, icon_pixmaps, title, description)| ToolTip {
            icon_name,
            icon_pixmaps: convert_icon_pixmaps(icon_pixmaps),
            title,
            description,
        },
    );
    let menu = take_property::<OwnedObjectPath>(&mut properties, "Menu")
        .map(|path| path.to_string());

    Ok(StatusNotifierItem {
        id,
        category: take_property(&mut properties, "Category"),
        identifier: take_property(&mut properties, "Id"),
        title: take_property(&mut properties, "Title"),
        status: take_property(&mut properties, "Status"),
        window_id: take_property(&mut properties, "WindowId"),
        icon_name: take_property(&mut properties, "IconName"),
        icon_pixmaps: take_property::<RawIconPixmaps>(&mut properties, "IconPixmap")
            .map(convert_icon_pixmaps)
            .unwrap_or_default(),
        overlay_icon_name: take_property(&mut properties, "OverlayIconName"),
        overlay_icon_pixmaps: take_property::<RawIconPixmaps>(
            &mut properties,
            "OverlayIconPixmap",
        )
        .map(convert_icon_pixmaps)
        .unwrap_or_default(),
        attention_icon_name: take_property(&mut properties, "AttentionIconName"),
        attention_icon_pixmaps: take_property::<RawIconPixmaps>(
            &mut properties,
            "AttentionIconPixmap",
        )
        .map(convert_icon_pixmaps)
        .unwrap_or_default(),
        attention_movie_name: take_property(&mut properties, "AttentionMovieName"),
        tool_tip,
        item_is_menu: take_property(&mut properties, "ItemIsMenu"),
        menu,
        icon_theme_path: take_property(&mut properties, "IconThemePath"),
    })
}

type RawIconPixmap = (i32, i32, Vec<u8>);
type RawIconPixmaps = Vec<RawIconPixmap>;
type RawToolTip = (String, RawIconPixmaps, String, String);

fn convert_icon_pixmaps(pixmaps: RawIconPixmaps) -> Vec<IconPixmap> {
    pixmaps
        .into_iter()
        .map(|(width, height, argb)| IconPixmap {
            width,
            height,
            argb,
        })
        .collect()
}

fn take_property<T>(properties: &mut HashMap<String, OwnedValue>, name: &str) -> Option<T>
where
    T: TryFrom<OwnedValue, Error = zbus::zvariant::Error>,
{
    let value = properties.remove(name)?;

    match T::try_from(value) {
        Ok(value) => Some(value),
        Err(error) => {
            log::debug!("Failed to decode status notifier property {name}: {error:#}");
            None
        }
    }
}

#[proxy(
    interface = "org.kde.StatusNotifierItem",
    assume_defaults = false,
    gen_blocking = false
)]
pub trait StatusNotifierItemInterface {
    #[zbus(property(emits_changed_signal = "false"))]
    fn category(&self) -> zbus::Result<String>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn id(&self) -> zbus::Result<String>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn title(&self) -> zbus::Result<String>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn status(&self) -> zbus::Result<String>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn window_id(&self) -> zbus::Result<u32>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn icon_name(&self) -> zbus::Result<String>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn icon_pixmap(&self) -> zbus::Result<RawIconPixmaps>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn overlay_icon_name(&self) -> zbus::Result<String>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn overlay_icon_pixmap(&self) -> zbus::Result<RawIconPixmaps>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn attention_icon_name(&self) -> zbus::Result<String>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn attention_icon_pixmap(&self) -> zbus::Result<RawIconPixmaps>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn attention_movie_name(&self) -> zbus::Result<String>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn tool_tip(&self) -> zbus::Result<RawToolTip>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn item_is_menu(&self) -> zbus::Result<bool>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn menu(&self) -> zbus::Result<OwnedObjectPath>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn icon_theme_path(&self) -> zbus::Result<String>;

    fn context_menu(&self, x: i32, y: i32) -> zbus::Result<()>;

    fn activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    fn secondary_activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    fn scroll(&self, delta: i32, orientation: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_title(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_icon(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_attention_icon(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_overlay_icon(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_tool_tip(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_status(&self, status: &str) -> zbus::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::StatusNotifierId;

    #[test]
    fn parses_registered_item_with_default_path() {
        let id = StatusNotifierId::from_registered_item("org.example.Item")
            .expect("valid status notifier item");

        assert_eq!(id.service(), "org.example.Item");
        assert_eq!(id.object_path(), "/StatusNotifierItem");
    }

    #[test]
    fn parses_registered_item_with_explicit_path() {
        let id = StatusNotifierId::from_registered_item(
            ":1.42/org/example/StatusNotifierItem",
        )
        .expect("valid status notifier item");

        assert_eq!(id.service(), ":1.42");
        assert_eq!(id.object_path(), "/org/example/StatusNotifierItem");
    }
}
