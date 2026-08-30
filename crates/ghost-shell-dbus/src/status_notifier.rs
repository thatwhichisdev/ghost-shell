use std::{collections::HashMap, process};

use anyhow::{Context as _, Result, anyhow};
use gpui::{App, AppContext as _, Context, Entity, EventEmitter};
use tokio::{sync::mpsc, task::JoinHandle};
use zbus::{Connection, fdo::RequestNameFlags};

use crate::{
    status_notifier_item::{
        StatusNotifierId, StatusNotifierItem, StatusNotifierItemClient,
        StatusNotifierItemEvent,
    },
    status_notifier_watcher::{
        StatusNotifierWatcherClient, StatusNotifierWatcherEvent,
        StatusNotifierWatcherService,
    },
};

const STATE_CHANNEL_CAPACITY: usize = 256;
const WATCHER_CHANNEL_CAPACITY: usize = 64;
const ITEM_CHANNEL_CAPACITY: usize = 256;

pub(crate) fn init(cx: &mut App) -> Entity<StatusNotifierState> {
    let state = cx.new(|_| StatusNotifierState::default());

    let (sender, mut receiver) = mpsc::channel(STATE_CHANNEL_CAPACITY);

    // D-Bus / zbus work MUST run on Tokio.
    gpui_tokio::Tokio::spawn(cx, async move {
        if let Err(error) = run(sender).await {
            log::error!("Status notifier integration stopped: {error:#}");
        }
    })
    .detach();

    // State mutations MUST run on GPUI.
    let state_handle = state.clone();

    cx.spawn(async move |cx| {
        while let Some(event) = receiver.recv().await {
            state_handle.update(cx, |state, cx| state.apply(event, cx))
        }
    })
    .detach();

    state
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatusNotifierEvent {
    Registered(StatusNotifierItem),
    Updated(StatusNotifierItem),
    Unregistered(StatusNotifierId),
}

#[derive(Default)]
pub struct StatusNotifierState {
    items: HashMap<StatusNotifierId, StatusNotifierItem>,
}

impl StatusNotifierState {
    pub fn items(&self) -> impl Iterator<Item = &StatusNotifierItem> {
        self.items.values()
    }

    #[must_use]
    pub fn item(&self, id: &StatusNotifierId) -> Option<&StatusNotifierItem> {
        self.items.get(id)
    }

    fn apply(&mut self, update: StatusNotifierStateUpdate, cx: &mut Context<Self>) {
        let event = match update {
            StatusNotifierStateUpdate::Registered(item) => {
                log::info!("registered tray item: {item:?}");

                let id = item.id.clone();

                self.items.insert(id.clone(), item.clone());

                StatusNotifierEvent::Registered(item)
            }

            StatusNotifierStateUpdate::Updated(item) => {
                let id = item.id.clone();

                if !self.items.contains_key(&id) {
                    log::debug!(
                        "Ignoring update for unregistered \
                         status notifier item {id}"
                    );

                    return;
                }

                self.items.insert(id.clone(), item.clone());

                StatusNotifierEvent::Updated(item)
            }

            StatusNotifierStateUpdate::Unregistered(id) => {
                if self.items.remove(&id).is_none() {
                    return;
                }

                StatusNotifierEvent::Unregistered(id)
            }
        };

        log::info!("dispatching dbus event: {event:?}");

        cx.emit(event);
        cx.notify();
    }
}

impl EventEmitter<StatusNotifierEvent> for StatusNotifierState {}

enum StatusNotifierStateUpdate {
    Registered(StatusNotifierItem),
    Updated(StatusNotifierItem),
    Unregistered(StatusNotifierId),
}

async fn run(state_updates: mpsc::Sender<StatusNotifierStateUpdate>) -> Result<()> {
    let connection = Connection::session()
        .await
        .context("failed to connect to D-Bus session bus")?;

    let host_service = format!("org.kde.StatusNotifierHost-{}", process::id(),);

    connection
        .request_name_with_flags(
            host_service.as_str(),
            RequestNameFlags::DoNotQueue.into(),
        )
        .await
        .context("failed to acquire StatusNotifierHost bus name")?;

    match StatusNotifierWatcherService::serve(&connection).await {
        Ok(()) => {
            log::info!("Ghost Shell is the status notifier watcher");
        }

        Err(zbus::Error::NameTaken) => {
            log::info!("Using existing status notifier watcher");
        }

        Err(error) => {
            return Err(error).context("failed to initialize status notifier watcher");
        }
    }

    let (watcher_events, mut watcher_event_receiver) =
        mpsc::channel(WATCHER_CHANNEL_CAPACITY);

    let (item_events, mut item_event_receiver) = mpsc::channel(ITEM_CHANNEL_CAPACITY);

    let watcher = StatusNotifierWatcherClient::new(&connection);

    let mut watcher_task = tokio::spawn(watcher.run(host_service, watcher_events));

    let mut item_tasks = HashMap::<StatusNotifierId, JoinHandle<Result<()>>>::new();

    let result = loop {
        tokio::select! {
            result = &mut watcher_task => {
                break match result {
                    Ok(Ok(())) => Err(anyhow!("status notifier watcher client stopped")),
                    Ok(Err(error)) => Err(error).context("status notifier watcher client failed"),
                    Err(error) => Err(error).context("status notifier watcher task failed"),
                }
            }

            Some(event) =
                watcher_event_receiver.recv() =>
            {
                if let Err(error) =
                    handle_watcher_event(
                        event,
                        &connection,
                        &item_events,
                        &state_updates,
                        &mut item_tasks,
                    )
                    .await
                {
                    break Err(error);
                }
            }

            Some(event) =
                item_event_receiver.recv() =>
            {
                if let Err(error) =
                    handle_item_event(
                        event,
                        &state_updates,
                        &item_tasks,
                    )
                    .await
                {
                    break Err(error);
                }
            }
        }
    };

    watcher_task.abort();

    for (_, task) in item_tasks {
        task.abort();
    }

    result
}

async fn handle_watcher_event(
    event: StatusNotifierWatcherEvent,
    connection: &Connection,
    item_events: &mpsc::Sender<StatusNotifierItemEvent>,
    state_updates: &mpsc::Sender<StatusNotifierStateUpdate>,
    item_tasks: &mut HashMap<StatusNotifierId, JoinHandle<Result<()>>>,
) -> Result<()> {
    match event {
        StatusNotifierWatcherEvent::Registered(registration) => {
            register_item(
                registration,
                connection,
                item_events,
                state_updates,
                item_tasks,
            )
            .await
        }

        StatusNotifierWatcherEvent::Unregistered(registration) => {
            unregister_item(registration, state_updates, item_tasks).await
        }
    }
}

async fn register_item(
    registration: String,
    connection: &Connection,
    item_events: &mpsc::Sender<StatusNotifierItemEvent>,
    state_updates: &mpsc::Sender<StatusNotifierStateUpdate>,
    item_tasks: &mut HashMap<StatusNotifierId, JoinHandle<Result<()>>>,
) -> Result<()> {
    let id = match StatusNotifierId::from_registered_item(&registration) {
        Ok(id) => id,

        Err(error) => {
            log::warn!(
                "Ignoring invalid status notifier \
                 registration {registration:?}: {error:#}"
            );

            return Ok(());
        }
    };

    if item_tasks
        .get(&id)
        .is_some_and(|task| !task.is_finished())
    {
        return Ok(());
    }

    // A previous client may have terminated before the
    // watcher emitted Unregistered. A new registration
    // should be allowed to replace it.
    item_tasks.remove(&id);

    let (item, task) = match StatusNotifierItemClient::spawn(
        connection,
        id.clone(),
        item_events.clone(),
    )
    .await
    {
        Ok(result) => result,

        Err(error) => {
            log::warn!(
                "Failed to initialize status notifier \
                     item {id}: {error:#}"
            );

            return Ok(());
        }
    };

    item_tasks.insert(id, task);

    state_updates
        .send(StatusNotifierStateUpdate::Registered(item))
        .await
        .context("status notifier state receiver stopped")
}

async fn unregister_item(
    registration: String,
    state_updates: &mpsc::Sender<StatusNotifierStateUpdate>,
    item_tasks: &mut HashMap<StatusNotifierId, JoinHandle<Result<()>>>,
) -> Result<()> {
    let id = match StatusNotifierId::from_registered_item(&registration) {
        Ok(id) => id,

        Err(error) => {
            log::warn!(
                "Ignoring invalid status notifier \
                 unregistration {registration:?}: \
                 {error:#}"
            );

            return Ok(());
        }
    };

    if let Some(task) = item_tasks.remove(&id) {
        task.abort();
    }

    state_updates
        .send(StatusNotifierStateUpdate::Unregistered(id))
        .await
        .context("status notifier state receiver stopped")
}

async fn handle_item_event(
    event: StatusNotifierItemEvent,
    state_updates: &mpsc::Sender<StatusNotifierStateUpdate>,
    item_tasks: &HashMap<StatusNotifierId, JoinHandle<Result<()>>>,
) -> Result<()> {
    match event {
        StatusNotifierItemEvent::Updated(item) => {
            // Prevent an already queued item update from
            // resurrecting an item after watcher
            // unregistration.
            if !item_tasks.contains_key(&item.id) {
                return Ok(());
            }

            state_updates
                .send(StatusNotifierStateUpdate::Updated(item))
                .await
                .context("status notifier state receiver stopped")
        }
    }
}
