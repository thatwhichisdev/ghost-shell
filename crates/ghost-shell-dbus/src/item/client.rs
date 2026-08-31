use std::{collections::HashMap, pin::Pin};

use anyhow::{Context as _, Result};
use futures_util::{Stream, StreamExt as _, stream::select_all};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use zbus::{
    Connection,
    fdo::PropertiesProxy,
    names::InterfaceName,
    proxy,
    zvariant::{OwnedObjectPath, OwnedValue},
};

use super::{IconPixmap, ItemEvent, StatusNotifierId, StatusNotifierItem, ToolTip};

const ITEM_INTERFACE: &str = "org.kde.StatusNotifierItem";

type RawIconPixmap = (i32, i32, Vec<u8>);
type RawIconPixmaps = Vec<RawIconPixmap>;
type RawToolTip = (String, RawIconPixmaps, String, String);

#[proxy(
    interface = "org.kde.StatusNotifierItem",
    assume_defaults = false,
    gen_blocking = false
)]
trait StatusNotifierItemInterface {
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

pub(super) struct StatusNotifierItemClient;

impl StatusNotifierItemClient {
    /// Starts listening to changes for `id`.
    ///
    /// Signal subscriptions are installed before the initial property
    /// snapshot is fetched, preventing a change from being lost between
    /// discovery and subscription.
    pub(super) async fn spawn(
        connection: &Connection,
        id: StatusNotifierId,
        events: mpsc::Sender<ItemEvent>,
    ) -> Result<(StatusNotifierItem, JoinHandle<Result<()>>)> {
        let connection = connection.clone();

        let (initial_sender, initial_receiver) = oneshot::channel();

        let task = tokio::spawn(Self::run(connection, id, initial_sender, events));

        match initial_receiver.await {
            Ok(item) => Ok((item, task)),

            Err(_) => match task.await {
                Ok(Err(error)) => Err(error),

                Ok(Ok(())) => {
                    anyhow::bail!(
                        "status notifier item client \
                         stopped before initialization"
                    );
                }

                Err(error) => Err(error.into()),
            },
        }
    }

    /// Fetches the current property snapshot without installing
    /// another update listener.
    pub(super) async fn fetch(
        connection: &Connection,
        id: StatusNotifierId,
    ) -> Result<StatusNotifierItem> {
        let properties = Self::properties_proxy(connection, &id).await?;

        Self::load(&properties, id).await
    }

    async fn run(
        connection: Connection,
        id: StatusNotifierId,
        initial: oneshot::Sender<StatusNotifierItem>,
        events: mpsc::Sender<ItemEvent>,
    ) -> Result<()> {
        let proxy = StatusNotifierItemInterfaceProxy::builder(&connection)
            .destination(id.service())?
            .path(id.object_path())?
            .build()
            .await
            .with_context(|| {
                format!(
                    "failed to create \
                     StatusNotifierItem proxy for {id}"
                )
            })?;

        let properties = Self::properties_proxy(&connection, &id).await?;

        // Subscribe first, fetch second.
        //
        // If the item changes while GetAll is in flight, the
        // corresponding signal is already queued for us.
        let changes: Vec<Pin<Box<dyn Stream<Item = ()> + Send + '_>>> = vec![
            Box::pin(proxy.receive_new_title().await?.map(|_| ())),
            Box::pin(proxy.receive_new_icon().await?.map(|_| ())),
            Box::pin(
                proxy
                    .receive_new_attention_icon()
                    .await?
                    .map(|_| ()),
            ),
            Box::pin(
                proxy
                    .receive_new_overlay_icon()
                    .await?
                    .map(|_| ()),
            ),
            Box::pin(
                proxy
                    .receive_new_tool_tip()
                    .await?
                    .map(|_| ()),
            ),
            Box::pin(proxy.receive_new_status().await?.map(|_| ())),
        ];

        let mut changes = select_all(changes);

        let item = Self::load(&properties, id.clone()).await?;

        if initial.send(item).is_err() {
            return Ok(());
        }

        while changes.next().await.is_some() {
            let item = match Self::load(&properties, id.clone()).await {
                Ok(item) => item,
                Err(error) => {
                    log::warn!("failed to refresh StatusNotifierItem {id}: {error:#}");
                    continue;
                }
            };

            if events
                .send(ItemEvent::Updated(item))
                .await
                .is_err()
            {
                return Ok(());
            }
        }

        Ok(())
    }

    async fn properties_proxy<'a>(
        connection: &'a Connection,
        id: &StatusNotifierId,
    ) -> Result<PropertiesProxy<'a>> {
        PropertiesProxy::builder(connection)
            .destination(id.service().to_owned())?
            .path(id.object_path().to_owned())?
            .build()
            .await
            .with_context(|| {
                format!(
                    "failed to create D-Bus properties \
                     proxy for {id}"
                )
            })
    }

    async fn load(
        properties: &PropertiesProxy<'_>,
        id: StatusNotifierId,
    ) -> Result<StatusNotifierItem> {
        let interface = InterfaceName::try_from(ITEM_INTERFACE)?;

        let mut properties = properties.get_all(interface).await?;

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
}

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
            log::debug!(
                "failed to decode StatusNotifierItem \
                 property {name}: {error:#}"
            );

            None
        }
    }
}
