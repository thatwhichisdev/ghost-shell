mod service;
pub(crate) const WATCHER_BUS_NAME: &str = "org.kde.StatusNotifierWatcher";
pub(crate) const WATCHER_OBJECT_PATH: &str = "/StatusNotifierWatcher";
use crate::{WatcherBackend, WatcherEvent};

pub struct Watcher;
impl WatcherBackend for Watcher {
    async fn watch(
        events: tokio::sync::mpsc::Sender<WatcherEvent>,
    ) -> anyhow::Result<()> {
        service::run(events).await?;
        Ok(())
    }
}
