mod client;
use anyhow::Result;
use client::DbusMenuClient;
use zbus::Connection;

use crate::{MenuBackend, MenuId, MenuSession};

#[derive(Clone)]
pub struct Menu {
    connection: Connection,
}
impl MenuBackend for Menu {
    async fn connect() -> Result<Self> {
        Ok(Self {
            connection: Connection::session().await?,
        })
    }
    async fn open(&self, id: MenuId) -> Result<MenuSession> {
        DbusMenuClient::open(&self.connection, id).await
    }
    async fn activate(&self, menu: MenuId, item_id: i32) -> Result<()> {
        DbusMenuClient::activate(&self.connection, &menu, item_id).await
    }
}
