use std::collections::HashMap;

use anyhow::{Context as _, Result};
use zbus::{Connection, proxy, zvariant::OwnedValue};

use super::{MenuId, MenuItem, MenuItemType, MenuLayout};

const ROOT_ITEM_ID: i32 = 0;
const UNLIMITED_RECURSION: i32 = -1;

type RawMenuItem = (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);

#[proxy(
    interface = "com.canonical.dbusmenu",
    assume_defaults = false,
    gen_blocking = false
)]
trait DbusMenuInterface {
    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<String>,
    ) -> zbus::Result<(u32, RawMenuItem)>;
}

pub(super) struct DbusMenuClient;

impl DbusMenuClient {
    pub(super) async fn fetch(connection: &Connection, id: MenuId) -> Result<MenuLayout> {
        let proxy = DbusMenuInterfaceProxy::builder(connection)
            .destination(id.service().to_owned())?
            .path(id.object_path().to_owned())?
            .build()
            .await
            .with_context(|| format!("failed to create D-Bus menu proxy for {id}"))?;

        let (revision, root) = proxy
            .get_layout(ROOT_ITEM_ID, UNLIMITED_RECURSION, Vec::new())
            .await
            .with_context(|| format!("failed to fetch D-Bus menu layout for {id}"))?;

        let root = decode_item(root)
            .with_context(|| format!("failed to decode D-Bus menu layout for {id}"))?;

        Ok(MenuLayout { id, revision, root })
    }
}

fn decode_item(raw: RawMenuItem) -> Result<MenuItem> {
    let (id, mut properties, children) = raw;

    let children = children
        .into_iter()
        .map(|child| {
            let child = RawMenuItem::try_from(child).with_context(|| {
                format!("failed to decode child of D-Bus menu item {id}")
            })?;

            decode_item(child)
        })
        .collect::<Result<Vec<_>>>()?;

    let item_type = match take_property::<String>(&mut properties, "type") {
        None => MenuItemType::Standard,

        Some(value) if value == "standard" => MenuItemType::Standard,

        Some(value) if value == "separator" => MenuItemType::Separator,

        Some(value) => MenuItemType::Other(value),
    };

    Ok(MenuItem {
        id,
        item_type,

        label: take_property(&mut properties, "label").unwrap_or_default(),

        enabled: take_property(&mut properties, "enabled").unwrap_or(true),

        visible: take_property(&mut properties, "visible").unwrap_or(true),

        icon_name: take_non_empty_string(&mut properties, "icon-name"),

        icon_data: take_property(&mut properties, "icon-data").unwrap_or_default(),

        shortcut: take_property(&mut properties, "shortcut").unwrap_or_default(),

        toggle_type: take_non_empty_string(&mut properties, "toggle-type"),

        toggle_state: take_property(&mut properties, "toggle-state").unwrap_or(-1),

        children_display: take_non_empty_string(&mut properties, "children-display"),

        children,
    })
}

fn take_non_empty_string(
    properties: &mut HashMap<String, OwnedValue>,
    name: &str,
) -> Option<String> {
    take_property::<String>(properties, name).filter(|value| !value.is_empty())
}

fn take_property<T>(properties: &mut HashMap<String, OwnedValue>, name: &str) -> Option<T>
where
    T: TryFrom<OwnedValue, Error = zbus::zvariant::Error>,
{
    let value = properties.remove(name)?;

    match T::try_from(value) {
        Ok(value) => Some(value),

        Err(error) => {
            log::debug!(
                "failed to decode D-Bus menu property \
                 {name}: {error:#}"
            );

            None
        }
    }
}
