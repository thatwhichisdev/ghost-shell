mod client;

use anyhow::{Context as _, Result};
use client::DbusMenuClient;
use ghost_shell_gpui::{App, AppContext as _, Context, Entity, Task};
use tokio::sync::{mpsc, oneshot};
use zbus::Connection;

const COMMAND_CHANNEL_CAPACITY: usize = 32;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MenuId {
    service: String,
    object_path: String,
}

impl MenuId {
    #[must_use]
    pub fn new(service: impl Into<String>, object_path: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            object_path: object_path.into(),
        }
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

impl std::fmt::Display for MenuId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}{}", self.service, self.object_path,)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MenuItemType {
    Standard,
    Separator,
    Other(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuItem {
    pub id: i32,
    pub item_type: MenuItemType,
    pub label: String,
    pub enabled: bool,
    pub visible: bool,

    pub icon_name: Option<String>,
    pub icon_data: Vec<u8>,

    pub shortcut: Vec<Vec<String>>,

    pub toggle_type: Option<String>,
    pub toggle_state: i32,

    pub children_display: Option<String>,
    pub children: Vec<MenuItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuLayout {
    pub id: MenuId,
    pub revision: u32,
    pub root: MenuItem,
}

impl MenuLayout {
    /// Returns the top-level visible menu entries.
    ///
    /// DBusMenu uses item `0` as the root container, so consumers
    /// generally want its children rather than rendering the root itself.
    #[must_use]
    pub fn items(&self) -> &[MenuItem] {
        &self.root.children
    }
}

/// A fresh layout and a subscription scoped to its visible lifetime.
/// Drop this session when the popup closes to release the D-Bus subscription.
pub struct MenuSession {
    pub layout: MenuLayout,
    invalidated: oneshot::Receiver<()>,
}

impl MenuSession {
    /// Waits until the layout/properties change or the signal connection closes.
    pub async fn invalidated(&mut self) {
        let _ = (&mut self.invalidated).await;
    }
}

pub struct Menu {
    commands: mpsc::Sender<MenuCommand>,
}

impl Menu {
    /// Prepares a remote menu for display and watches for changes.
    pub fn open(&self, id: MenuId, cx: &mut Context<Self>) -> Task<Result<MenuSession>> {
        let commands = self.commands.clone();

        cx.spawn(async move |_this, _cx| {
            let (reply, response) = oneshot::channel();

            commands
                .send(MenuCommand::Open { id, reply })
                .await
                .context("D-Bus menu client stopped")?;

            response
                .await
                .context("D-Bus menu client dropped menu open request")?
        })
    }

    pub fn activate(
        &self,
        menu: MenuId,
        item_id: i32,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        let commands = self.commands.clone();

        cx.spawn(async move |_this, _cx| {
            let (reply, response) = oneshot::channel();

            commands
                .send(MenuCommand::Activate {
                    menu,
                    item_id,
                    reply,
                })
                .await
                .context("D-Bus menu client stopped")?;

            response
                .await
                .context("D-Bus menu client dropped activation request")?
        })
    }
}

enum MenuCommand {
    Open {
        id: MenuId,
        reply: oneshot::Sender<Result<MenuSession>>,
    },
    Activate {
        menu: MenuId,
        item_id: i32,
        reply: oneshot::Sender<Result<()>>,
    },
}

pub(crate) fn init(cx: &mut App) -> Entity<Menu> {
    let (commands, command_receiver) = mpsc::channel(COMMAND_CHANNEL_CAPACITY);

    let menu = cx.new(|_| Menu { commands });

    ghost_shell_tokio::Tokio::spawn(cx, async move {
        if let Err(error) = run(command_receiver).await {
            log::error!("D-Bus menu client stopped: {error:#}");
        }
    })
    .detach();

    menu
}

async fn run(mut commands: mpsc::Receiver<MenuCommand>) -> Result<()> {
    let connection = Connection::session()
        .await
        .context("failed to connect D-Bus menu client to session bus")?;

    while let Some(command) = commands.recv().await {
        match command {
            MenuCommand::Open { id, reply } => {
                let result = DbusMenuClient::open(&connection, id).await;

                let _ = reply.send(result);
            }

            MenuCommand::Activate {
                menu,
                item_id,
                reply,
            } => {
                let result = DbusMenuClient::activate(&connection, &menu, item_id).await;

                let _ = reply.send(result);
            }
        }
    }

    Ok(())
}
