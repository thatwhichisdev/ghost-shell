mod client;

use std::{collections::HashMap, fmt};

use anyhow::{Context as _, Result, bail};
use client::StatusNotifierItemClient;
use gpui::{App, AppContext as _, Context, Entity, EventEmitter, Task};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use zbus::Connection;

const COMMAND_CHANNEL_CAPACITY: usize = 64;
const EVENT_CHANNEL_CAPACITY: usize = 256;

const DEFAULT_ITEM_PATH: &str = "/StatusNotifierItem";

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StatusNotifierId {
    service: String,
    object_path: String,
}

impl StatusNotifierId {
    pub fn from_registration(registration: &str) -> Result<Self> {
        let (service, object_path) = match registration.find('/') {
            Some(index) if index > 0 => (&registration[..index], &registration[index..]),

            Some(_) => {
                bail!(
                    "status notifier registration \
                         is missing a service name"
                );
            }

            None => (registration, DEFAULT_ITEM_PATH),
        };

        if service.is_empty() {
            bail!(
                "status notifier registration \
                 is missing a service name"
            );
        }

        Ok(Self {
            service: service.to_owned(),
            object_path: object_path.to_owned(),
        })
    }

    #[must_use]
    pub fn service(&self) -> &str {
        &self.service
    }

    #[must_use]
    pub fn object_path(&self) -> &str {
        &self.object_path
    }
}

impl fmt::Display for StatusNotifierId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", self.service, self.object_path,)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IconPixmap {
    pub width: i32,
    pub height: i32,
    pub argb: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolTip {
    pub icon_name: String,
    pub icon_pixmaps: Vec<IconPixmap>,
    pub title: String,
    pub description: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusNotifierItem {
    pub id: StatusNotifierId,

    pub category: Option<String>,
    pub identifier: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub window_id: Option<u32>,

    pub icon_name: Option<String>,
    pub icon_pixmaps: Vec<IconPixmap>,

    pub overlay_icon_name: Option<String>,
    pub overlay_icon_pixmaps: Vec<IconPixmap>,

    pub attention_icon_name: Option<String>,
    pub attention_icon_pixmaps: Vec<IconPixmap>,
    pub attention_movie_name: Option<String>,

    pub tool_tip: Option<ToolTip>,
    pub item_is_menu: Option<bool>,
    pub menu: Option<String>,

    /// KDE extension used by real-world StatusNotifierItem
    /// implementations.
    pub icon_theme_path: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ItemEvent {
    Updated(StatusNotifierItem),
}

pub struct Item {
    commands: mpsc::Sender<ItemCommand>,
}

impl Item {
    /// Discovers the current properties of a StatusNotifierItem and
    /// ensures that Ghost is listening for future updates from it.
    pub fn discover(
        &self,
        registration: impl Into<String>,
        cx: &mut Context<Self>,
    ) -> Task<Result<StatusNotifierItem>> {
        let registration = registration.into();
        let commands = self.commands.clone();

        cx.spawn(async move |_this, _cx| {
            let id = StatusNotifierId::from_registration(&registration)?;

            let (reply, response) = oneshot::channel();

            commands
                .send(ItemCommand::Discover { id, reply })
                .await
                .context("status notifier item client stopped")?;

            response.await.context(
                "status notifier item client \
                     dropped discovery request",
            )?
        })
    }
}

impl EventEmitter<ItemEvent> for Item {}

enum ItemCommand {
    Discover {
        id: StatusNotifierId,
        reply: oneshot::Sender<Result<StatusNotifierItem>>,
    },
}

/// Initializes the StatusNotifierItem client facade.
pub(crate) fn init(cx: &mut App) -> Entity<Item> {
    let (commands, command_receiver) = mpsc::channel(COMMAND_CHANNEL_CAPACITY);
    let (events, mut event_receiver) = mpsc::channel(EVENT_CHANNEL_CAPACITY);

    let item = cx.new(|_| Item { commands });

    gpui_tokio::Tokio::spawn(cx, async move {
        if let Err(error) = run(command_receiver, events).await {
            log::error!(
                "status notifier item client stopped: \
                 {error:#}"
            );
        }
    })
    .detach();

    let item_handle = item.clone();

    cx.spawn(async move |cx| {
        while let Some(event) = event_receiver.recv().await {
            item_handle.update(cx, |_item, cx| {
                cx.emit(event);
            });
        }
    })
    .detach();

    item
}

async fn run(
    mut commands: mpsc::Receiver<ItemCommand>,
    events: mpsc::Sender<ItemEvent>,
) -> Result<()> {
    let connection = Connection::session().await.context(
        "failed to connect StatusNotifierItem \
             client to D-Bus session bus",
    )?;

    let mut clients = HashMap::<StatusNotifierId, JoinHandle<()>>::new();

    while let Some(command) = commands.recv().await {
        // A remote application may have disappeared since
        // its client was created. Clean completed listeners
        // opportunistically before processing the request.
        clients.retain(|_, task| !task.is_finished());

        match command {
            ItemCommand::Discover { id, reply } => {
                let result = if clients.contains_key(&id) {
                    StatusNotifierItemClient::fetch(&connection, id).await
                } else {
                    match StatusNotifierItemClient::spawn(
                        &connection,
                        id.clone(),
                        events.clone(),
                    )
                    .await
                    {
                        Ok((item, client_task)) => {
                            let task_id = id.clone();

                            let task = tokio::spawn(async move {
                                match client_task.await {
                                    Ok(Ok(())) => {}

                                    Ok(Err(error)) => {
                                        log::debug!(
                                            "status notifier \
                                                     item client \
                                                     {task_id} \
                                                     stopped: \
                                                     {error:#}"
                                        );
                                    }

                                    Err(error) => {
                                        log::debug!(
                                            "status notifier \
                                                     item client \
                                                     {task_id} \
                                                     task stopped: \
                                                     {error}"
                                        );
                                    }
                                }
                            });

                            clients.insert(id, task);

                            Ok(item)
                        }

                        Err(error) => Err(error),
                    }
                };

                // The tray may have disappeared before the
                // D-Bus request completed. That is not an
                // item-client failure.
                let _ = reply.send(result);
            }
        }
    }

    for (_, task) in clients {
        task.abort();
    }

    Ok(())
}
