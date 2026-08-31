mod service;

use std::collections::HashSet;

use gpui::{App, AppContext as _, Context, Entity, EventEmitter};
use tokio::sync::mpsc;

const EVENT_CHANNEL_CAPACITY: usize = 64;

pub(crate) const WATCHER_BUS_NAME: &str = "org.kde.StatusNotifierWatcher";
pub(crate) const WATCHER_OBJECT_PATH: &str = "/StatusNotifierWatcher";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WatcherEvent {
    Registered(String),
    Unregistered(String),
}

#[derive(Debug, Default)]
pub struct Watcher {
    items: HashSet<String>,
}

impl Watcher {
    /// Returns all currently registered StatusNotifier objects.
    pub fn items(&self) -> impl Iterator<Item = &str> {
        self.items.iter().map(String::as_str)
    }

    /// Returns whether an object is currently registered.
    #[must_use]
    pub fn contains(&self, item: &str) -> bool {
        self.items.contains(item)
    }

    fn apply(&mut self, event: WatcherEvent, cx: &mut Context<Self>) {
        let changed = match &event {
            WatcherEvent::Registered(item) => self.items.insert(item.clone()),
            WatcherEvent::Unregistered(item) => self.items.remove(item),
        };

        if changed {
            cx.emit(event);
            cx.notify();
        }
    }
}

impl EventEmitter<WatcherEvent> for Watcher {}

/// Initializes the StatusNotifierWatcher service and its GPUI-facing state.
pub(crate) fn init(cx: &mut App) -> Entity<Watcher> {
    let watcher = cx.new(|_| Watcher::default());
    let (event_sender, mut event_receiver) = mpsc::channel(EVENT_CHANNEL_CAPACITY);

    gpui_tokio::Tokio::spawn(cx, async move {
        if let Err(error) = service::run(event_sender).await {
            log::error!("status notifier watcher stopped: {error:#}");
        }
    })
    .detach();

    let watcher_handle = watcher.clone();
    cx.spawn(async move |cx| {
        while let Some(event) = event_receiver.recv().await {
            watcher_handle.update(cx, |watcher, cx| watcher.apply(event, cx))
        }
    })
    .detach();

    watcher
}
