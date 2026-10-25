mod item;
mod menu;
mod watcher;
pub mod zbus_backend;

use std::future::Future;

use anyhow::Result;
pub use item::{IconPixmap, ItemEvent, StatusNotifierId, StatusNotifierItem, ToolTip};
pub use menu::{MenuId, MenuItem, MenuItemType, MenuLayout, MenuSession};
use tokio::{sync::mpsc, task::JoinHandle};
pub use watcher::WatcherEvent;

pub type Item = zbus_backend::Item;
pub type Menu = zbus_backend::Menu;
pub type Watcher = zbus_backend::Watcher;

pub trait ItemBackend: Clone + Send + Sync + Sized {
    fn connect() -> impl Future<Output = Result<Self>> + Send;
    fn fetch(
        &self,
        id: StatusNotifierId,
    ) -> impl Future<Output = Result<StatusNotifierItem>> + Send;
    /// Subscribe before taking the snapshot. The caller must abort the returned task on removal.
    fn discover(
        &self,
        id: StatusNotifierId,
        events: mpsc::Sender<ItemEvent>,
    ) -> impl Future<Output = Result<(StatusNotifierItem, JoinHandle<Result<()>>)>> + Send;
}

pub trait MenuBackend: Clone + Send + Sync + Sized {
    fn connect() -> impl Future<Output = Result<Self>> + Send;
    fn open(&self, id: MenuId) -> impl Future<Output = Result<MenuSession>> + Send;
    fn activate(
        &self,
        menu: MenuId,
        item_id: i32,
    ) -> impl Future<Output = Result<()>> + Send;
}

pub trait WatcherBackend {
    fn watch(
        events: mpsc::Sender<WatcherEvent>,
    ) -> impl Future<Output = Result<()>> + Send;
}

pub async fn watch(events: mpsc::Sender<WatcherEvent>) -> Result<()> {
    Watcher::watch(events).await
}
