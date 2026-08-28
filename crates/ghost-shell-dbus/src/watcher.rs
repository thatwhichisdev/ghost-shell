use std::collections::HashSet;

use tokio::sync::mpsc;
use zbus::{fdo, interface, message::Header, object_server::SignalEmitter, proxy};

pub(crate) const WATCHER_BUS_NAME: &str = "org.kde.StatusNotifierWatcher";
pub(crate) const WATCHER_OBJECT_PATH: &str = "/StatusNotifierWatcher";
const DEFAULT_ITEM_PATH: &str = "/StatusNotifierItem";

#[derive(Debug)]
pub(crate) enum WatcherEvent {
    Registered(String),
    Unregistered(String),
}

#[proxy(
    interface = "org.kde.StatusNotifierWatcher",
    default_service = "org.kde.StatusNotifierWatcher",
    default_path = "/StatusNotifierWatcher",
    gen_blocking = false
)]
pub(crate) trait StatusNotifierWatcherClient {
    fn register_status_notifier_host(&self, service: &str) -> zbus::Result<()>;

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> zbus::Result<Vec<String>>;

    #[zbus(signal)]
    fn status_notifier_item_registered(&self, service: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    fn status_notifier_item_unregistered(&self, service: &str) -> zbus::Result<()>;
}

#[derive(Debug)]
pub struct StatusNotifierWatcher {
    items: HashSet<String>,
    hosts: HashSet<String>,
    event_sender: mpsc::Sender<WatcherEvent>,
}

impl StatusNotifierWatcher {
    pub(crate) fn new(host: String, event_sender: mpsc::Sender<WatcherEvent>) -> Self {
        Self {
            items: HashSet::new(),
            hosts: HashSet::from([host]),
            event_sender,
        }
    }

    pub(crate) async fn remove_items_for_service(
        &mut self,
        service: &str,
        emitter: &SignalEmitter<'_>,
    ) -> zbus::Result<Vec<String>> {
        let prefix = format!("{service}/");
        let mut removed: Vec<String> = self
            .items
            .extract_if(|item| item.starts_with(&prefix))
            .collect();
        removed.sort();

        for item in &removed {
            Self::status_notifier_item_unregistered(emitter, item).await?;
        }
        if !removed.is_empty() {
            self.registered_status_notifier_items_changed(emitter)
                .await?;
        }

        Ok(removed)
    }
}

#[interface(name = "org.kde.StatusNotifierWatcher", introspection_docs = false)]
impl StatusNotifierWatcher {
    async fn register_status_notifier_item(
        &mut self,
        service_or_path: &str,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> fdo::Result<()> {
        if service_or_path.is_empty() {
            return Err(fdo::Error::InvalidArgs(
                "missing service or object path".into(),
            ));
        }

        let item = if service_or_path.starts_with('/') {
            let sender = header
                .sender()
                .ok_or_else(|| fdo::Error::InvalidArgs("missing sender".into()))?;

            format!("{sender}{service_or_path}")
        } else if service_or_path.contains('/') {
            service_or_path.to_owned()
        } else {
            format!("{service_or_path}{DEFAULT_ITEM_PATH}")
        };

        if self.items.insert(item.clone()) {
            emitter
                .status_notifier_item_registered(&item)
                .await
                .map_err(fdo::Error::ZBus)?;
            self.registered_status_notifier_items_changed(&emitter)
                .await
                .map_err(fdo::Error::ZBus)?;
            self.event_sender
                .send(WatcherEvent::Registered(item))
                .await
                .map_err(|_| fdo::Error::Failed("tray service stopped".into()))?;
        }

        Ok(())
    }

    async fn register_status_notifier_host(
        &mut self,
        service: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> fdo::Result<()> {
        if self.hosts.insert(service.to_owned()) {
            emitter
                .status_notifier_host_registered()
                .await
                .map_err(fdo::Error::ZBus)?;
        }

        Ok(())
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        let mut items: Vec<String> = self.items.iter().cloned().collect();
        items.sort();
        items
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn is_status_notifier_host_registered(&self) -> bool {
        !self.hosts.is_empty()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    pub(crate) async fn status_notifier_item_registered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub(crate) async fn status_notifier_item_unregistered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub(crate) async fn status_notifier_host_registered(
        emitter: &SignalEmitter<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub(crate) async fn status_notifier_host_unregistered(
        emitter: &SignalEmitter<'_>,
    ) -> zbus::Result<()>;
}
