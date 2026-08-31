use std::collections::HashSet;

use futures_util::StreamExt as _;
use tokio::sync::mpsc;
use zbus::{
    Connection,
    fdo::{self, DBusProxy, RequestNameFlags},
    interface,
    message::Header,
    names::BusName,
    object_server::SignalEmitter,
};

use super::{WATCHER_BUS_NAME, WATCHER_OBJECT_PATH, WatcherEvent};

const DEFAULT_ITEM_PATH: &str = "/StatusNotifierItem";

pub(super) struct StatusNotifierWatcherService {
    items: HashSet<String>,
    events: mpsc::Sender<WatcherEvent>,
}

impl StatusNotifierWatcherService {
    fn new(events: mpsc::Sender<WatcherEvent>) -> Self {
        Self {
            items: HashSet::new(),
            events,
        }
    }

    async fn dispatch(&self, event: WatcherEvent) {
        if self.events.send(event).await.is_err() {
            log::warn!(
                "status notifier watcher event receiver \
                 has been dropped"
            );
        }
    }

    async fn remove_items_for_service(
        &mut self,
        service: &str,
        emitter: &SignalEmitter<'_>,
    ) -> zbus::Result<()> {
        let prefix = format!("{service}/");

        let removed = self
            .items
            .iter()
            .filter(|item| item.starts_with(&prefix))
            .cloned()
            .collect::<Vec<_>>();

        if removed.is_empty() {
            return Ok(());
        }

        for item in removed {
            self.items.remove(&item);

            Self::status_notifier_item_unregistered(emitter, &item).await?;

            self.dispatch(WatcherEvent::Unregistered(item))
                .await;
        }

        self.registered_status_notifier_items_changed(emitter)
            .await?;

        Ok(())
    }
}

#[interface(name = "org.kde.StatusNotifierWatcher", introspection_docs = false)]
impl StatusNotifierWatcherService {
    async fn register_status_notifier_item(
        &mut self,
        service_or_path: &str,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> fdo::Result<()> {
        if service_or_path.is_empty() {
            return Err(fdo::Error::InvalidArgs(
                "missing service or object path".into(),
            ));
        }

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

        if !self.items.insert(item.clone()) {
            return Ok(());
        }

        Self::status_notifier_item_registered(&emitter, &item).await?;

        self.registered_status_notifier_items_changed(&emitter)
            .await?;

        self.dispatch(WatcherEvent::Registered(item))
            .await;

        Ok(())
    }

    async fn register_status_notifier_host(&self, _service: &str) -> fdo::Result<()> {
        // Ghost is the StatusNotifier host.
        //
        // No host registry is required. This mirrors the
        // behavior of the current KDE watcher.
        Ok(())
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        let mut items = self
            .items
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        items.sort();

        items
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property(emits_changed_signal = "const"))]
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

/// Runs Ghost's `org.kde.StatusNotifierWatcher` service.
pub(super) async fn run(events: mpsc::Sender<WatcherEvent>) -> zbus::Result<()> {
    let connection = Connection::session().await?;
    let dbus = DBusProxy::new(&connection).await?;

    // Subscribe before exposing the watcher so we cannot
    // miss a service disappearing immediately after it
    // registers.
    let mut owner_changes = dbus.receive_name_owner_changed().await?;

    connection
        .object_server()
        .at(
            WATCHER_OBJECT_PATH,
            StatusNotifierWatcherService::new(events),
        )
        .await?;

    connection
        .request_name_with_flags(WATCHER_BUS_NAME, RequestNameFlags::DoNotQueue.into())
        .await?;

    while let Some(signal) = owner_changes.next().await {
        let args = signal.args()?;

        // We only care about a name which previously had
        // an owner. This also handles direct ownership
        // replacement of a well-known name.
        if args.old_owner().is_none() {
            continue;
        }

        let service = args.name().to_string();

        let watcher = connection
            .object_server()
            .interface::<_, StatusNotifierWatcherService>(WATCHER_OBJECT_PATH)
            .await?;

        let emitter = watcher.signal_emitter().clone();

        watcher
            .get_mut()
            .await
            .remove_items_for_service(&service, &emitter)
            .await?;
    }

    Ok(())
}
