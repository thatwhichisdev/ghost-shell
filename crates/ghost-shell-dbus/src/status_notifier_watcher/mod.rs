mod client;
mod service;

pub(crate) use client::StatusNotifierWatcherClient;
pub(crate) use service::StatusNotifierWatcherService;

pub(crate) const WATCHER_SERVICE: &str = "org.kde.StatusNotifierWatcher";
pub(crate) const WATCHER_PATH: &str = "/StatusNotifierWatcher";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum StatusNotifierWatcherEvent {
    Registered(String),
    Unregistered(String),
}
