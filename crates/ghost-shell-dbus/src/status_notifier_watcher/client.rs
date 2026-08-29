use futures_util::StreamExt as _;
use tokio::sync::mpsc;
use zbus::{Connection, proxy};

use super::StatusNotifierWatcherEvent;

#[proxy(
    interface = "org.kde.StatusNotifierWatcher",
    default_service = "org.kde.StatusNotifierWatcher",
    default_path = "/StatusNotifierWatcher",
    gen_blocking = false
)]
trait StatusNotifierWatcher {
    fn register_status_notifier_host(&self, service: &str) -> zbus::Result<()>;

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> zbus::Result<Vec<String>>;

    #[zbus(signal)]
    fn status_notifier_item_registered(&self, service: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    fn status_notifier_item_unregistered(&self, service: &str) -> zbus::Result<()>;
}

#[derive(Clone)]
pub(crate) struct StatusNotifierWatcherClient {
    connection: Connection,
}

impl StatusNotifierWatcherClient {
    pub(crate) fn new(connection: &Connection) -> Self {
        Self {
            connection: connection.clone(),
        }
    }

    pub(crate) async fn run(
        self,
        host_service: String,
        events: mpsc::Sender<StatusNotifierWatcherEvent>,
    ) -> zbus::Result<()> {
        let proxy = StatusNotifierWatcherProxy::new(&self.connection).await?;

        // Subscribe before taking the initial snapshot so registrations
        // cannot disappear into the gap between GetAll and subscription.
        let mut registered = proxy
            .receive_status_notifier_item_registered()
            .await?;

        let mut unregistered = proxy
            .receive_status_notifier_item_unregistered()
            .await?;

        proxy
            .register_status_notifier_host(&host_service)
            .await?;

        for item in proxy
            .registered_status_notifier_items()
            .await?
        {
            if events
                .send(StatusNotifierWatcherEvent::Registered(item))
                .await
                .is_err()
            {
                return Ok(());
            }
        }

        loop {
            tokio::select! {
                signal = registered.next() => {
                    let Some(signal) = signal else {
                        return Ok(());
                    };

                    let item =
                        signal.args()?.service().to_string();

                    if events
                        .send(
                            StatusNotifierWatcherEvent::Registered(
                                item,
                            ),
                        )
                        .await
                        .is_err()
                    {
                        return Ok(());
                    }
                }

                signal = unregistered.next() => {
                    let Some(signal) = signal else {
                        return Ok(());
                    };

                    let item =
                        signal.args()?.service().to_string();

                    if events
                        .send(
                            StatusNotifierWatcherEvent::Unregistered(
                                item,
                            ),
                        )
                        .await
                        .is_err()
                    {
                        return Ok(());
                    }
                }
            }
        }
    }
}
