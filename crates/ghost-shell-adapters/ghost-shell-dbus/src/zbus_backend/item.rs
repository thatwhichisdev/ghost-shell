mod client;
use snafu::ResultExt as _;
pub(super) type Result<T> = std::result::Result<T, Error>;
use client::StatusNotifierItemClient;
use tokio::{sync::mpsc, task::JoinHandle};
use zbus::Connection;

use crate::{ItemBackend, ItemEvent, StatusNotifierId, StatusNotifierItem};

#[derive(Clone)]
pub struct Item {
    connection: Connection,
}
impl ItemBackend for Item {
    async fn connect() -> Result<Self> {
        Ok(Self {
            connection: Connection::session()
                .await
                .context(ConnectSnafu)?,
        })
    }
    async fn fetch(&self, id: StatusNotifierId) -> Result<StatusNotifierItem> {
        StatusNotifierItemClient::fetch(&self.connection, id).await
    }
    async fn discover(
        &self,
        id: StatusNotifierId,
        events: mpsc::Sender<ItemEvent>,
    ) -> Result<(StatusNotifierItem, JoinHandle<Result<()>>)> {
        StatusNotifierItemClient::spawn(&self.connection, id, events).await
    }
}

#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub(super)))]
pub enum Error {
    #[snafu(context(false), display("D-Bus operation failed"))]
    Bus { source: zbus::Error },
    #[snafu(context(false), display("D-Bus daemon request failed"))]
    Daemon { source: zbus::fdo::Error },
    #[snafu(context(false), display("Invalid D-Bus name"))]
    Name { source: zbus::names::Error },
    #[snafu(display("Could not connect to the session bus for tray items"))]
    Connect { source: zbus::Error },
    #[snafu(display("Could not create tray item proxy for {id}"))]
    Proxy {
        id: crate::StatusNotifierId,
        source: zbus::Error,
    },
    #[snafu(display("Could not read tray item properties for {id}"))]
    Properties {
        id: crate::StatusNotifierId,
        source: zbus::fdo::Error,
    },
    #[snafu(display("Tray item client stopped before initialization"))]
    InitializationStopped,
    #[snafu(context(false), display("Tray item listener task failed"))]
    Task { source: tokio::task::JoinError },
}
