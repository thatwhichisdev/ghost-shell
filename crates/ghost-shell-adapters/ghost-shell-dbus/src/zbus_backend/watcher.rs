mod service;

pub(crate) const WATCHER_BUS_NAME: &str = "org.kde.StatusNotifierWatcher";
pub(crate) const WATCHER_OBJECT_PATH: &str = "/StatusNotifierWatcher";

use crate::{WatcherBackend, WatcherEvent};

pub struct Watcher;

impl WatcherBackend for Watcher {
    async fn watch(events: tokio::sync::mpsc::Sender<WatcherEvent>) -> Result<()> {
        service::run(events).await?;
        Ok(())
    }
}

pub(super) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub(super)))]
pub enum Error {
    #[snafu(context(false), display("D-Bus operation failed"))]
    Bus { source: zbus::Error },
    #[snafu(context(false), display("D-Bus daemon request failed"))]
    Daemon { source: zbus::fdo::Error },
    #[snafu(context(false), display("Invalid D-Bus name"))]
    Name { source: zbus::names::Error },
    #[snafu(display("Session bus did not assign a unique name"))]
    MissingUniqueName,
    #[snafu(display("Session bus owner stream ended"))]
    OwnerStreamEnded,
    #[snafu(display("Tray watcher signal stream ended"))]
    SignalStreamEnded,
    #[snafu(context(false), display("Tray watcher event receiver closed"))]
    ReceiverClosed {
        source: tokio::sync::mpsc::error::SendError<crate::WatcherEvent>,
    },
}
