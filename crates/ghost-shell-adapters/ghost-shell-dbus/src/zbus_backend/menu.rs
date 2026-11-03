mod client;
use snafu::ResultExt as _;
pub(super) type Result<T> = std::result::Result<T, Error>;
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
            connection: Connection::session()
                .await
                .context(ConnectSnafu)?,
        })
    }
    async fn open(&self, id: MenuId) -> Result<MenuSession> {
        DbusMenuClient::open(&self.connection, id).await
    }
    async fn activate(&self, menu: MenuId, item_id: i32) -> Result<()> {
        DbusMenuClient::activate(&self.connection, &menu, item_id).await
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
    #[snafu(display("Could not connect to the session bus for tray menus"))]
    Connect { source: zbus::Error },
    #[snafu(display("Could not create D-Bus menu proxy for {id}"))]
    Proxy {
        id: crate::MenuId,
        source: zbus::Error,
    },
    #[snafu(display("Could not watch D-Bus menu {id}"))]
    Watch {
        id: crate::MenuId,
        source: zbus::Error,
    },
    #[snafu(display("Could not fetch D-Bus menu layout for {id}"))]
    Layout {
        id: crate::MenuId,
        source: zbus::Error,
    },
    #[snafu(display("Could not decode child of D-Bus menu item {id}"))]
    Decode {
        id: i32,
        source: zbus::zvariant::Error,
    },
    #[snafu(display("Could not activate D-Bus menu item {item_id} for {menu}"))]
    Activate {
        menu: crate::MenuId,
        item_id: i32,
        source: zbus::Error,
    },
}
