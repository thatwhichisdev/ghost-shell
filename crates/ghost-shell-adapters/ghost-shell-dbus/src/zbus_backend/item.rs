mod client;
use anyhow::Result;
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
            connection: Connection::session().await?,
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
