use std::{collections::HashMap, pin::Pin};

use anyhow::{Context as _, Result};
use futures_util::{Stream, StreamExt as _, stream::select_all};
use gpui::{App, AppContext as _, Context, Entity, EventEmitter, Global};
use tokio::{sync::mpsc, task::JoinHandle};
use zbus::{
    connection::Builder,
    fdo::{DBusProxy, RequestNameFlags},
    object_server::InterfaceRef,
};

use crate::{
    StatusNotifierId, StatusNotifierItem, StatusNotifierItemInterfaceProxy,
    fetch_status_notifier_item,
    watcher::{
        StatusNotifierWatcher, StatusNotifierWatcherClientProxy, WATCHER_BUS_NAME,
        WATCHER_OBJECT_PATH, WatcherEvent,
    },
};

const EVENT_CHANNEL_CAPACITY: usize = 256;
const REGISTRATION_CHANNEL_CAPACITY: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatusNotifierEvent {
    Added(StatusNotifierItem),
    Updated(StatusNotifierItem),
    Removed(StatusNotifierId),
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

    fn apply(&mut self, event: StatusNotifierEvent, cx: &mut Context<Self>) {
        match &event {
            StatusNotifierEvent::Added(item) | StatusNotifierEvent::Updated(item) => {
                self.items
                    .insert(item.id.clone(), item.clone());
            }
            StatusNotifierEvent::Removed(id) => {
                self.items.remove(id);
            }
        }

        cx.emit(event);
        cx.notify();
    }
}

impl EventEmitter<StatusNotifierEvent> for StatusNotifierState {}

pub struct DbusIntegration {
    status_notifier: Entity<StatusNotifierState>,
}

impl DbusIntegration {
    #[must_use]
    pub fn status_notifier(&self) -> &Entity<StatusNotifierState> {
        &self.status_notifier
    }
}

impl Global for DbusIntegration {}

pub fn init(cx: &mut App) {
    let status_notifier = cx.new(|_| StatusNotifierState::default());
    cx.set_global(DbusIntegration {
        status_notifier: status_notifier.clone(),
    });

    let (event_sender, mut event_receiver) = mpsc::channel(EVENT_CHANNEL_CAPACITY);

    gpui_tokio::Tokio::spawn(cx, async move {
        if let Err(error) = run_status_notifier_service(event_sender).await {
            log::error!("Status notifier service stopped: {error:#}");
        }
    })
    .detach();

    cx.spawn(async move |cx| {
        while let Some(event) = event_receiver.recv().await {
            status_notifier.update(cx, |state, cx| state.apply(event, cx));
        }
    })
    .detach();
}

async fn run_status_notifier_service(
    event_sender: mpsc::Sender<StatusNotifierEvent>,
) -> Result<()> {
    let host_name = format!("org.kde.StatusNotifierHost-{}", std::process::id());
    let (watcher_event_sender, mut watcher_event_receiver) =
        mpsc::channel(REGISTRATION_CHANNEL_CAPACITY);
    let connection = Builder::session()?
        .name(host_name.clone())?
        .build()
        .await
        .context("failed to connect to the session bus")?;

    let dbus = DBusProxy::new(&connection).await?;
    let watcher_name = WATCHER_BUS_NAME.try_into()?;
    let watcher = if dbus.name_has_owner(watcher_name).await? {
        spawn_external_watcher_listener(
            connection.clone(),
            host_name,
            watcher_event_sender.clone(),
        );
        log::info!("Using the existing status notifier watcher");
        None
    } else {
        match start_as_watcher_owner(
            &connection,
            host_name.clone(),
            watcher_event_sender.clone(),
        )
        .await
        {
            Ok(watcher) => {
                log::info!("Ghost Shell is the status notifier watcher");
                Some(watcher)
            }
            Err(zbus::Error::NameTaken) => {
                connection
                    .object_server()
                    .remove::<StatusNotifierWatcher, _>(WATCHER_OBJECT_PATH)
                    .await?;
                spawn_external_watcher_listener(
                    connection.clone(),
                    host_name,
                    watcher_event_sender.clone(),
                );
                log::info!("Using the status notifier watcher that won startup");
                None
            }
            Err(error) => return Err(error.into()),
        }
    };

    let mut owner_changes = dbus.receive_name_owner_changed().await?;
    let mut item_tasks: HashMap<StatusNotifierId, JoinHandle<()>> = HashMap::new();

    loop {
        tokio::select! {
            watcher_event = watcher_event_receiver.recv() => {
                let Some(watcher_event) = watcher_event else {
                    break;
                };

                match watcher_event {
                    WatcherEvent::Registered(registration) => {
                        let id = match StatusNotifierId::from_registered_item(&registration) {
                            Ok(id) => id,
                            Err(error) => {
                                log::warn!(
                                    "Ignoring invalid status notifier registration {registration:?}: {error:#}"
                                );
                                continue;
                            }
                        };

                        if item_tasks.get(&id).is_some_and(|task| !task.is_finished()) {
                            continue;
                        }

                        if let Some(finished_task) = item_tasks.remove(&id)
                            && let Err(error) = finished_task.await
                        {
                            log::debug!(
                                "Previous status notifier task for {id} stopped: {error:#}"
                            );
                        }

                        let connection = connection.clone();
                        let event_sender = event_sender.clone();
                        let task_id = id.clone();
                        let task = tokio::spawn(async move {
                            if let Err(error) = watch_status_notifier_item(
                                connection,
                                task_id.clone(),
                                event_sender,
                            )
                            .await
                            {
                                log::warn!(
                                    "Stopped watching status notifier item {task_id}: {error:#}"
                                );
                            }
                        });
                        item_tasks.insert(id, task);
                    }
                    WatcherEvent::Unregistered(registration) => {
                        let id = StatusNotifierId::from_registered_item(&registration)?;
                        remove_status_notifier_item(
                            id,
                            &mut item_tasks,
                            &event_sender,
                        )
                        .await?;
                    }
                }
            }
            owner_change = owner_changes.next() => {
                let Some(owner_change) = owner_change else {
                    break;
                };
                let arguments = owner_change.args()?;
                if arguments.old_owner().is_none() {
                    continue;
                }

                let service = arguments.name().as_str();
                let removed = if let Some(watcher) = &watcher {
                    let removed = {
                        let mut watcher_state = watcher.get_mut().await;
                        watcher_state
                            .remove_items_for_service(
                                service,
                                watcher.signal_emitter(),
                            )
                            .await?
                    };
                    removed
                        .into_iter()
                        .map(|item| StatusNotifierId::from_registered_item(&item))
                        .collect::<Result<Vec<_>>>()?
                } else {
                    item_tasks
                        .keys()
                        .filter(|id| id.service() == service)
                        .cloned()
                        .collect()
                };

                for id in removed {
                    remove_status_notifier_item(
                        id,
                        &mut item_tasks,
                        &event_sender,
                    )
                    .await?;
                }
            }
        }
    }

    Ok(())
}

async fn start_as_watcher_owner(
    connection: &zbus::Connection,
    host_name: String,
    watcher_event_sender: mpsc::Sender<WatcherEvent>,
) -> zbus::Result<InterfaceRef<StatusNotifierWatcher>> {
    connection
        .object_server()
        .at(
            WATCHER_OBJECT_PATH,
            StatusNotifierWatcher::new(host_name, watcher_event_sender),
        )
        .await?;
    connection
        .request_name_with_flags(WATCHER_BUS_NAME, RequestNameFlags::DoNotQueue.into())
        .await?;

    let watcher = connection
        .object_server()
        .interface::<_, StatusNotifierWatcher>(WATCHER_OBJECT_PATH)
        .await?;
    StatusNotifierWatcher::status_notifier_host_registered(watcher.signal_emitter())
        .await?;

    Ok(watcher)
}

fn spawn_external_watcher_listener(
    connection: zbus::Connection,
    host_name: String,
    watcher_event_sender: mpsc::Sender<WatcherEvent>,
) {
    tokio::spawn(async move {
        if let Err(error) =
            listen_to_external_watcher(connection, &host_name, watcher_event_sender).await
        {
            log::error!("External status notifier watcher stopped: {error:#}");
        }
    });
}

async fn listen_to_external_watcher(
    connection: zbus::Connection,
    host_name: &str,
    watcher_event_sender: mpsc::Sender<WatcherEvent>,
) -> Result<()> {
    let watcher = StatusNotifierWatcherClientProxy::new(&connection).await?;
    let mut registered = watcher
        .receive_status_notifier_item_registered()
        .await?;
    let mut unregistered = watcher
        .receive_status_notifier_item_unregistered()
        .await?;

    watcher
        .register_status_notifier_host(host_name)
        .await?;
    for item in watcher
        .registered_status_notifier_items()
        .await?
    {
        watcher_event_sender
            .send(WatcherEvent::Registered(item))
            .await
            .context("status notifier watcher event receiver stopped")?;
    }

    loop {
        tokio::select! {
            signal = registered.next() => {
                let Some(signal) = signal else {
                    break;
                };
                let item = signal.args()?.service().to_string();
                watcher_event_sender
                    .send(WatcherEvent::Registered(item))
                    .await
                    .context("status notifier watcher event receiver stopped")?;
            }
            signal = unregistered.next() => {
                let Some(signal) = signal else {
                    break;
                };
                let item = signal.args()?.service().to_string();
                watcher_event_sender
                    .send(WatcherEvent::Unregistered(item))
                    .await
                    .context("status notifier watcher event receiver stopped")?;
            }
        }
    }

    Ok(())
}

async fn remove_status_notifier_item(
    id: StatusNotifierId,
    item_tasks: &mut HashMap<StatusNotifierId, JoinHandle<()>>,
    event_sender: &mpsc::Sender<StatusNotifierEvent>,
) -> Result<()> {
    let Some(task) = item_tasks.remove(&id) else {
        return Ok(());
    };

    task.abort();
    event_sender
        .send(StatusNotifierEvent::Removed(id))
        .await
        .context("status notifier event receiver stopped")
}

async fn watch_status_notifier_item(
    connection: zbus::Connection,
    id: StatusNotifierId,
    event_sender: mpsc::Sender<StatusNotifierEvent>,
) -> Result<()> {
    let proxy = StatusNotifierItemInterfaceProxy::builder(&connection)
        .destination(id.service())?
        .path(id.object_path())?
        .build()
        .await?;
    let properties_proxy = zbus::fdo::PropertiesProxy::builder(&connection)
        .destination(id.service())?
        .path(id.object_path())?
        .build()
        .await?;

    let changes: Vec<Pin<Box<dyn Stream<Item = ()> + Send + '_>>> = vec![
        Box::pin(proxy.receive_new_title().await?.map(|_| ())),
        Box::pin(proxy.receive_new_icon().await?.map(|_| ())),
        Box::pin(
            proxy
                .receive_new_attention_icon()
                .await?
                .map(|_| ()),
        ),
        Box::pin(
            proxy
                .receive_new_overlay_icon()
                .await?
                .map(|_| ()),
        ),
        Box::pin(
            proxy
                .receive_new_tool_tip()
                .await?
                .map(|_| ()),
        ),
        Box::pin(proxy.receive_new_status().await?.map(|_| ())),
        Box::pin(
            properties_proxy
                .receive_properties_changed()
                .await?
                .map(|_| ()),
        ),
    ];
    let mut changes = select_all(changes);

    let item = match fetch_status_notifier_item(&connection, id.clone()).await {
        Ok(item) => item,
        Err(error) => {
            log::warn!(
                "Failed to load status notifier item {id}, publishing an empty item: {error:#}"
            );
            StatusNotifierItem::empty(id.clone())
        }
    };

    event_sender
        .send(StatusNotifierEvent::Added(item))
        .await
        .context("status notifier event receiver stopped")?;

    while changes.next().await.is_some() {
        match fetch_status_notifier_item(&connection, id.clone()).await {
            Ok(item) => {
                event_sender
                    .send(StatusNotifierEvent::Updated(item))
                    .await
                    .context("status notifier event receiver stopped")?;
            }
            Err(error) => {
                log::warn!("Failed to refresh status notifier item {id}: {error:#}");
            }
        }
    }

    Ok(())
}
