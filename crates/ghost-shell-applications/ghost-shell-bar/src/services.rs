use std::collections::HashMap;

use futures::{SinkExt as _, Stream};
use ghost_shell_dbus::{
    Item, ItemBackend as _, ItemEvent, Menu, MenuBackend as _, StatusNotifierId,
    WatcherEvent,
};
use tokio::sync::mpsc;

#[derive(Clone)]
pub enum Event {
    Item(ghost_shell_dbus::StatusNotifierItem),
    Removed(StatusNotifierId),
    Menu(Menu),
    Error(String),
}

pub fn events() -> impl Stream<Item = Event> {
    iced::stream::channel::<Event>(128, async |mut output| {
        let (sender, mut receiver) = mpsc::channel(128);
        let mut workers = tokio::task::JoinSet::new();
        let tray_sender = sender.clone();
        workers.spawn(async move {
            if let Err(error) = tray(tray_sender.clone()).await {
                if tray_sender
                    .send(Event::Error(format!("Tray: {error:#}")))
                    .await
                    .is_err()
                {
                    log::debug!("Tray receiver closed");
                }
            }
        });
        drop(sender);
        loop {
            tokio::select! {
                event = receiver.recv() => { let Some(event) = event else { break; }; if output.send(event).await.is_err() { break; } }
                result = workers.join_next(), if !workers.is_empty() => { if let Some(Err(error)) = result { if output.send(Event::Error(format!("Service worker: {error}"))).await.is_err() { break; } } }
            }
        }
    })
}

async fn tray(output: mpsc::Sender<Event>) -> anyhow::Result<()> {
    let item = Item::connect().await?;
    output
        .send(Event::Menu(Menu::connect().await?))
        .await?;
    let (registrations, mut registered) = mpsc::channel(64);
    let (updates, mut updated) = mpsc::channel(128);
    let watcher = ghost_shell_dbus::watch(registrations);
    tokio::pin!(watcher);
    let mut clients = HashMap::new();
    // Abort handles are retained so dropping the service also stops signal listeners.
    struct Clients(Vec<tokio::task::AbortHandle>);
    impl Drop for Clients {
        fn drop(&mut self) {
            for task in &self.0 {
                task.abort();
            }
        }
    }
    let mut cleanup = Clients(Vec::new());
    loop {
        tokio::select! {
            result = &mut watcher => return Ok(result?),
            registration = registered.recv() => match registration {
                Some(WatcherEvent::Registered(registration)) => {
                    let id = StatusNotifierId::from_registration(&registration)?;
                    if clients.contains_key(&id) { continue; }
                    match item.discover(id.clone(), updates.clone()).await {
                        Ok((snapshot, task)) => { cleanup.0.retain(|task| !task.is_finished()); cleanup.0.push(task.abort_handle()); clients.insert(id, task); output.send(Event::Item(snapshot)).await?; }
                        Err(error) => { output.send(Event::Error(format!("Tray discovery: {error:#}"))).await?; }
                    }
                }
                Some(WatcherEvent::Unregistered(registration)) => {
                    let id = StatusNotifierId::from_registration(&registration)?;
                    if let Some(task) = clients.remove(&id) { task.abort(); }
                    output.send(Event::Removed(id)).await?;
                }
                None => return Ok(()),
            },
            update = updated.recv() => if let Some(ItemEvent::Updated(item)) = update { if clients.contains_key(&item.id) { output.send(Event::Item(item)).await?; } }
        }
    }
}
