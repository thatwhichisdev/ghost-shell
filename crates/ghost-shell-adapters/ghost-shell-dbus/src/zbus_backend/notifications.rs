mod service;

use std::collections::BTreeMap;

use snafu::{OptionExt as _, ResultExt as _};
use tokio::{
    sync::{mpsc, oneshot},
    time::Instant,
};
use zbus::{Connection, fdo::RequestNameFlags, object_server::SignalEmitter};

const NAME: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";

#[derive(Clone, Debug)]
pub struct Notification {
    pub id: u32,
    pub revision: u64,
    pub app_name: String,
    pub app_icon: String,
    pub desktop_entry: Option<String>,
    pub summary: String,
    pub body: String,
}

#[derive(Clone)]
pub enum NotificationEvent {
    Ready(Notifications),
    Updated(Notification),
    Closed(u32),
}

#[derive(Clone)]
pub struct Notifications(mpsc::Sender<Command>);

impl Notifications {
    pub async fn dismiss(&self, id: u32) -> Result<(), Error> {
        let (reply, response) = oneshot::channel();
        self.0
            .send(Command::Close(id, 2, reply))
            .await
            .map_err(|_| ServiceStoppedSnafu.build())?;
        response.await.context(ReplyClosedSnafu)?
    }
}

enum Command {
    Notify(Notification, i32, oneshot::Sender<Result<u32, Error>>),
    Close(u32, u32, oneshot::Sender<Result<(), Error>>),
}

struct Entry {
    deadline: Option<Instant>,
}

#[derive(Default)]
struct Store {
    entries: BTreeMap<u32, Entry>,
    next: u32,
    revision: u64,
}

impl Store {
    fn insert(
        &mut self,
        notification: &mut Notification,
        timeout: i32,
    ) -> Result<(), Error> {
        if !self.entries.contains_key(&notification.id) {
            if self.entries.len() >= 256 {
                return LimitReachedSnafu.fail();
            }
            self.next = self
                .next
                .checked_add(1)
                .context(IdsExhaustedSnafu)?;
            notification.id = self.next;
        }
        self.revision = self.revision.wrapping_add(1);
        notification.revision = self.revision;
        self.entries.insert(
            notification.id,
            Entry {
                deadline: (timeout > 0).then(|| {
                    Instant::now() + std::time::Duration::from_millis(timeout as u64)
                }),
            },
        );
        Ok(())
    }
}

pub async fn notifications(events: mpsc::Sender<NotificationEvent>) -> Result<(), Error> {
    serve(Connection::session().await?, events).await
}

async fn serve(
    connection: Connection,
    events: mpsc::Sender<NotificationEvent>,
) -> Result<(), Error> {
    let (sender, mut commands) = mpsc::channel(64);
    connection
        .object_server()
        .at(PATH, service::Service(sender.clone()))
        .await?;
    connection
        .request_name_with_flags(NAME, RequestNameFlags::DoNotQueue.into())
        .await
        .context(NameTakenSnafu)?;
    let emitter = SignalEmitter::new(&connection, PATH)?;
    events
        .send(NotificationEvent::Ready(Notifications(sender)))
        .await?;
    let mut store = Store::default();
    let mut bus = zbus::MessageStream::from(&connection);
    use futures_util::StreamExt as _;
    loop {
        let deadline = store
            .entries
            .values()
            .filter_map(|entry| entry.deadline)
            .min();
        let expiration = async {
            match deadline {
                Some(deadline) => tokio::time::sleep_until(deadline).await,
                None => std::future::pending::<()>().await,
            }
        };
        tokio::select! {
            _ = events.closed() => return Ok(()),
            message = bus.next() => match message {
                Some(Ok(_)) => {},
                Some(Err(error)) => return Err(error.into()),
                None => return DisconnectedSnafu.fail(),
            },
            _ = expiration => {
                let expired: Vec<_> = store.entries.iter().filter_map(|(id, entry)|
                    entry.deadline.is_some_and(|deadline| deadline <= Instant::now()).then_some(*id)
                ).collect();
                for id in expired {
                    store.entries.remove(&id);
                    events.send(NotificationEvent::Closed(id)).await?;
                    service::Service::notification_closed(&emitter, id, 1).await?;
                }
            },
            command = commands.recv() => match command {
                Some(Command::Notify(mut notification, timeout, reply)) => {
                    let result = store.insert(&mut notification, timeout);
                    match result {
                        Ok(()) => {
                            let id = notification.id;
                            events.send(NotificationEvent::Updated(notification)).await?;
                            if reply.send(Ok(id)).is_err() { log::debug!("Notification caller disconnected"); }
                        }
                        Err(error) => if reply.send(Err(error)).is_err() { log::debug!("Notification caller disconnected"); },
                    }
                }
                Some(Command::Close(id, reason, reply)) => {
                    let result = if store.entries.remove(&id).is_some() {
                        events.send(NotificationEvent::Closed(id)).await?;
                        service::Service::notification_closed(&emitter, id, reason).await?;
                        Ok(())
                    } else {
                        MissingNotificationSnafu { id }.fail()
                    };
                    if reply.send(result).is_err() { log::debug!("Notification caller disconnected"); }
                }
                None => return Ok(()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn notification(id: u32) -> Notification {
        Notification {
            id,
            revision: 0,
            app_name: String::new(),
            app_icon: String::new(),
            desktop_entry: None,
            summary: String::new(),
            body: String::new(),
        }
    }
    #[test]
    fn replacement_keeps_id_and_resets_deadline() {
        let mut store = Store::default();
        let mut first = notification(0);
        assert!(store.insert(&mut first, 1000).is_ok());
        let revision = first.revision;
        assert!(
            store
                .entries
                .get(&first.id)
                .is_some_and(|entry| entry.deadline.is_some())
        );
        assert!(store.insert(&mut first, 0).is_ok());
        assert_eq!(first.id, 1);
        assert!(first.revision > revision);
        assert!(
            store
                .entries
                .get(&first.id)
                .is_some_and(|entry| entry.deadline.is_none())
        );
        let mut missing = notification(50);
        assert!(store.insert(&mut missing, -1).is_ok());
        assert_eq!(missing.id, 2);
    }
    struct TestBus(std::process::Child);
    impl Drop for TestBus {
        fn drop(&mut self) {
            if let Err(error) = self.0.kill() {
                eprintln!("Test bus cleanup: {error}");
            }
            if let Err(error) = self.0.wait() {
                eprintln!("Test bus wait: {error}");
            }
        }
    }

    #[tokio::test]
    async fn protocol_replacement_dismissal_expiry_and_name_conflict()
    -> Result<(), Box<dyn std::error::Error>> {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            use std::io::BufRead as _;
            use futures_util::StreamExt as _;
            use zbus::{connection::Builder, zvariant::OwnedValue};
            let mut bus = TestBus(std::process::Command::new("dbus-daemon")
                .args(["--session", "--nofork", "--print-address=1"])
                .stdout(std::process::Stdio::piped()).spawn()?);
            let mut address = String::new();
            std::io::BufReader::new(bus.0.stdout.take().ok_or("Missing bus stdout")?)
                .read_line(&mut address)?;
            let server = Builder::address(address.trim())?.build().await?;
            let client = Builder::address(address.trim())?.build().await?;
            let (events, mut received) = mpsc::channel(64);
            let task = tokio::spawn(serve(server, events));
            let Some(NotificationEvent::Ready(handle)) = received.recv().await else { panic!("Server not ready") };
            let proxy = zbus::Proxy::new(&client, NAME, PATH, NAME).await?;
            let capabilities: Vec<String> = proxy.call("GetCapabilities", &()).await?;
            assert_eq!(capabilities, vec!["body", "persistence"]);
            let information: (String, String, String, String) = proxy.call("GetServerInformation", &()).await?;
            assert_eq!(information.0, "Ghost Shell");
            let mut closed = proxy.receive_signal("NotificationClosed").await?;
            let notify = |replaces_id, timeout| (
                "Test", replaces_id, "test-icon", "Summary", "Body",
                Vec::<String>::new(), std::collections::HashMap::<String, OwnedValue>::new(), timeout,
            );
            let first: u32 = proxy.call("Notify", &notify(0u32, 0i32)).await?;
            assert!(first > 0);
            assert!(matches!(received.recv().await, Some(NotificationEvent::Updated(item)) if item.id == first));
            let replacement: u32 = proxy.call("Notify", &notify(first, 0)).await?;
            assert_eq!(replacement, first);
            assert!(matches!(received.recv().await, Some(NotificationEvent::Updated(item)) if item.id == first));
            handle.dismiss(first).await?;
            assert!(matches!(received.recv().await, Some(NotificationEvent::Closed(id)) if id == first));
            let signal = closed.next().await.ok_or("Missing dismissal signal")?;
            assert_eq!(signal.body().deserialize::<(u32,u32)>()?, (first, 2));
            let second: u32 = proxy.call("Notify", &notify(0, 0)).await?;
            received.recv().await.ok_or("Missing update")?;
            proxy.call::<_,_,()>("CloseNotification", &(second,)).await?;
            assert!(matches!(received.recv().await, Some(NotificationEvent::Closed(id)) if id == second));
            let signal = closed.next().await.ok_or("Missing close signal")?;
            assert_eq!(signal.body().deserialize::<(u32,u32)>()?, (second, 3));
            assert!(proxy.call::<_,_,()>("CloseNotification", &(second,)).await.is_err());
            let third: u32 = proxy.call("Notify", &notify(0, 25)).await?;
            received.recv().await.ok_or("Missing expiring update")?;
            assert!(matches!(received.recv().await, Some(NotificationEvent::Closed(id)) if id == third));
            let signal = closed.next().await.ok_or("Missing expiration signal")?;
            assert_eq!(signal.body().deserialize::<(u32,u32)>()?, (third, 1));
            let other = Builder::address(address.trim())?.build().await?;
            let (events, _receiver) = mpsc::channel(1);
            assert!(serve(other, events).await.is_err());
            drop(received);
            task.await??;
            Ok::<_, Box<dyn std::error::Error>>(())
        }).await?
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
    #[snafu(display(
        "Could not own org.freedesktop.Notifications; stop any other notification daemon"
    ))]
    NameTaken { source: zbus::Error },
    #[snafu(display("Notification service stopped"))]
    ServiceStopped,
    #[snafu(display("Notification service dropped the reply"))]
    ReplyClosed {
        source: tokio::sync::oneshot::error::RecvError,
    },
    #[snafu(display("Notification limit reached (256 unread notifications)"))]
    LimitReached,
    #[snafu(display("Notification IDs exhausted"))]
    IdsExhausted,
    #[snafu(display("Notification {id} does not exist"))]
    MissingNotification { id: u32 },
    #[snafu(display("Notification session bus disconnected"))]
    Disconnected,
    #[snafu(context(false), display("Notification event receiver closed"))]
    ReceiverClosed {
        source: tokio::sync::mpsc::error::SendError<NotificationEvent>,
    },
}
