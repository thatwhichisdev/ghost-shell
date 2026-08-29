use futures_util::StreamExt as _;
use zbus::{
    Connection,
    fdo::{self, DBusProxy, NameOwnerChangedStream, RequestNameFlags},
    interface,
    message::Header,
    names::BusName,
    object_server::SignalEmitter,
};

use super::{WATCHER_PATH, WATCHER_SERVICE};

const DEFAULT_ITEM_PATH: &str = "/StatusNotifierItem";

#[interface(name = "org.kde.StatusNotifierWatcher", introspection_docs = false)]
impl StatusNotifierWatcherService {
    async fn register_status_notifier_item(
        &mut self,
        service_or_path: &str,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> fdo::Result<()> {
        let (service, path) = if service_or_path.starts_with('/') {
            let sender = header
                .sender()
                .ok_or_else(|| fdo::Error::InvalidArgs("missing D-Bus sender".into()))?;

            (sender.to_string(), service_or_path.to_owned())
        } else {
            (service_or_path.to_owned(), DEFAULT_ITEM_PATH.to_owned())
        };

        let bus_name = BusName::try_from(service.as_str())
            .map_err(|error| fdo::Error::InvalidArgs(error.to_string()))?;

        let dbus = DBusProxy::new(connection).await?;

        if !dbus.name_has_owner(bus_name).await? {
            return Ok(());
        }

        let item = format!("{service}{path}");

        if self.items.contains(&item) {
            return Ok(());
        }

        self.items.push(item.clone());

        self.registered_status_notifier_items_changed(&emitter)
            .await?;

        Self::status_notifier_item_registered(&emitter, &item).await?;

        Ok(())
    }

    async fn register_status_notifier_host(&self, _service: &str) -> fdo::Result<()> {
        // Deliberately mirrors current KDE behavior.
        //
        // Ghost is the visual host, so we don't need to maintain an
        // arbitrary registry of other host implementations.
        Ok(())
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.clone()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(
        emitter: &SignalEmitter<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_unregistered(
        emitter: &SignalEmitter<'_>,
    ) -> zbus::Result<()>;
}

#[derive(Debug, Default)]
pub(crate) struct StatusNotifierWatcherService {
    items: Vec<String>,
}

impl StatusNotifierWatcherService {
    pub(crate) async fn serve(connection: &Connection) -> zbus::Result<()> {
        // Subscribe before acquiring the watcher name. Once the name becomes
        // visible, applications are allowed to register immediately.
        let dbus = DBusProxy::new(connection).await?;
        let owner_changes = dbus.receive_name_owner_changed().await?;

        connection
            .object_server()
            .at(WATCHER_PATH, Self::default())
            .await?;

        if let Err(error) = connection
            .request_name_with_flags(WATCHER_SERVICE, RequestNameFlags::DoNotQueue.into())
            .await
        {
            let _ = connection
                .object_server()
                .remove::<Self, _>(WATCHER_PATH)
                .await;

            return Err(error);
        }

        let connection = connection.clone();

        tokio::spawn(async move {
            if let Err(error) = Self::watch_name_owners(connection, owner_changes).await {
                log::error!("status notifier watcher owner monitor failed: {error}");
            }
        });

        Ok(())
    }

    async fn watch_name_owners(
        connection: Connection,
        mut owner_changes: NameOwnerChangedStream,
    ) -> zbus::Result<()> {
        while let Some(signal) = owner_changes.next().await {
            let args = signal.args()?;

            // We only care about names which previously had an owner.
            //
            // This also handles an owner being replaced directly:
            // old_owner = Some(...), new_owner = Some(...).
            if args.old_owner().is_none() {
                continue;
            }

            let service = args.name().to_string();

            let watcher = connection
                .object_server()
                .interface::<_, Self>(WATCHER_PATH)
                .await?;

            watcher
                .get_mut()
                .await
                .unregister_service(&service, watcher.signal_emitter())
                .await?;
        }

        Ok(())
    }

    async fn unregister_service(
        &mut self,
        service: &str,
        emitter: &SignalEmitter<'_>,
    ) -> zbus::Result<()> {
        let prefix = format!("{service}/");
        let mut removed = Vec::new();

        self.items.retain(|item| {
            if item.starts_with(&prefix) {
                removed.push(item.clone());
                false
            } else {
                true
            }
        });

        if removed.is_empty() {
            return Ok(());
        }

        self.registered_status_notifier_items_changed(emitter)
            .await?;

        for item in removed {
            Self::status_notifier_item_unregistered(emitter, &item).await?;
        }

        Ok(())
    }
}
