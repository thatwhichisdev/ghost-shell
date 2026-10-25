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

/// Hosts a watcher when available, otherwise follows the existing watcher.
pub async fn run(events: mpsc::Sender<WatcherEvent>) -> anyhow::Result<()> {
    let connection = Connection::session().await?;
    let dbus = DBusProxy::new(&connection).await?;
    let mut owner_changes = dbus.receive_name_owner_changed().await?;

    connection
        .object_server()
        .at(
            WATCHER_OBJECT_PATH,
            StatusNotifierWatcherService::new(events.clone()),
        )
        .await?;

    let unique_name = connection
        .unique_name()
        .ok_or_else(|| anyhow::anyhow!("session bus did not assign a unique name"))?;
    let host_name = format!(
        "org.kde.StatusNotifierHost.ghost_shell_{}",
        unique_name.as_str().replace([':', '.'], "_")
    );
    connection
        .request_name(host_name.as_str())
        .await?;

    loop {
        if events.is_closed() {
            return Ok(());
        }
        match connection
            .request_name_with_flags(
                WATCHER_BUS_NAME,
                RequestNameFlags::DoNotQueue.into(),
            )
            .await
        {
            Ok(_) => break,
            Err(zbus::Error::NameTaken) => {}
            Err(error) => return Err(error.into()),
        }

        let owner = match dbus
            .get_name_owner(BusName::try_from(WATCHER_BUS_NAME)?)
            .await
        {
            Ok(owner) => owner,
            Err(fdo::Error::NameHasNoOwner(_)) => continue,
            Err(error) => return Err(error.into()),
        };
        let mut items = HashSet::new();
        let result = {
            // Pin the proxy to this owner so a replacement cannot mix its
            // snapshot or signals with the previous watcher's registrations.
            let follower =
                follow(&connection, owner.as_str(), &host_name, &events, &mut items);
            tokio::pin!(follower);
            loop {
                tokio::select! {
                    result = &mut follower => break result,
                    _ = events.closed() => return Ok(()),
                    signal = owner_changes.next() => {
                        let Some(signal) = signal else { return Err(anyhow::anyhow!("session bus owner stream ended")); };
                        let args = signal.args()?;
                        if args.name().as_str() == WATCHER_BUS_NAME
                            && args.old_owner().as_ref().is_some_and(|previous| previous.as_str() == owner.as_str())
                        {
                            break Ok(());
                        }
                    }
                }
            }
        };
        for item in items {
            events
                .send(WatcherEvent::Unregistered(item))
                .await?;
        }
        if let Err(error) = result {
            // A disappearing owner can fail an in-flight property/method call
            // before its NameOwnerChanged signal is consumed.
            match dbus
                .get_name_owner(BusName::try_from(WATCHER_BUS_NAME)?)
                .await
            {
                Ok(current) if current != owner => continue,
                Err(fdo::Error::NameHasNoOwner(_)) => continue,
                _ => return Err(error),
            }
        }
    }

    loop {
        let signal = tokio::select! {
            _ = events.closed() => return Ok(()),
            signal = owner_changes.next() => signal,
        };
        let Some(signal) = signal else {
            return Err(anyhow::anyhow!("session bus owner stream ended"));
        };
        let args = signal.args()?;
        if args.old_owner().is_none() {
            continue;
        }
        let watcher = connection
            .object_server()
            .interface::<_, StatusNotifierWatcherService>(WATCHER_OBJECT_PATH)
            .await?;
        let emitter = watcher.signal_emitter().clone();
        watcher
            .get_mut()
            .await
            .remove_items_for_service(args.name().as_str(), &emitter)
            .await?;
    }
}

async fn follow(
    connection: &Connection,
    owner: &str,
    host_name: &str,
    events: &mpsc::Sender<WatcherEvent>,
    items: &mut HashSet<String>,
) -> anyhow::Result<()> {
    let proxy =
        zbus::Proxy::new(connection, owner, WATCHER_OBJECT_PATH, WATCHER_BUS_NAME)
            .await?;
    // Subscribe before the snapshot so registrations during Get are not lost.
    let mut signals = proxy.receive_all_signals().await?;
    proxy
        .call::<_, _, ()>("RegisterStatusNotifierHost", &(host_name,))
        .await?;
    let snapshot: Vec<String> = proxy
        .get_property("RegisteredStatusNotifierItems")
        .await?;
    for item in snapshot {
        if items.insert(item.clone()) {
            events
                .send(WatcherEvent::Registered(item))
                .await?;
        }
    }
    while let Some(signal) = signals.next().await {
        let header = signal.header();
        let Some(member) = header.member() else {
            continue;
        };
        match member.as_str() {
            "StatusNotifierItemRegistered" => {
                let (item,): (String,) = signal.body().deserialize()?;
                if items.insert(item.clone()) {
                    events
                        .send(WatcherEvent::Registered(item))
                        .await?;
                }
            }
            "StatusNotifierItemUnregistered" => {
                let (item,): (String,) = signal.body().deserialize()?;
                if items.remove(&item) {
                    events
                        .send(WatcherEvent::Unregistered(item))
                        .await?;
                }
            }
            _ => {}
        }
    }
    Err(anyhow::anyhow!("tray watcher signal stream ended"))
}
