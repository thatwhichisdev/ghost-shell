use std::collections::HashMap;

use tokio::sync::{mpsc, oneshot};
use zbus::{fdo, interface, object_server::SignalEmitter, zvariant::OwnedValue};

use super::{Command, Notification};

pub(super) struct Service(pub(super) mpsc::Sender<Command>);

#[interface(name = "org.freedesktop.Notifications", introspection_docs = false)]
impl Service {
    fn get_capabilities(&self) -> Vec<&str> {
        vec!["body", "persistence"]
    }

    fn get_server_information(&self) -> (&str, &str, &str, &str) {
        (
            "Ghost Shell",
            "Ghost Shell",
            env!("CARGO_PKG_VERSION"),
            "1.2",
        )
    }

    async fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> fdo::Result<u32> {
        if expire_timeout < -1
            || app_name.len() + app_icon.len() + summary.len() + body.len() > 65536
        {
            return Err(fdo::Error::InvalidArgs(
                "Invalid timeout or notification exceeds 64 KiB".into(),
            ));
        }
        // Actions and markup are deliberately not advertised in GetCapabilities.
        drop(actions);
        let desktop_entry = hints
            .get("desktop-entry")
            .and_then(|value| <&str>::try_from(value).ok())
            .map(str::to_owned);
        let transient = hints
            .get("transient")
            .and_then(|value| bool::try_from(value).ok())
            .unwrap_or(false);
        let timeout = if transient && expire_timeout == -1 {
            5000
        } else {
            expire_timeout
        };
        let (reply, response) = oneshot::channel();
        self.0
            .send(Command::Notify(
                Notification {
                    id: replaces_id,
                    revision: 0,
                    app_name,
                    app_icon,
                    desktop_entry,
                    summary,
                    body,
                },
                timeout,
                reply,
            ))
            .await
            .map_err(|error| fdo::Error::Failed(error.to_string()))?;
        response
            .await
            .map_err(|error| fdo::Error::Failed(error.to_string()))?
            .map_err(|error| {
                fdo::Error::Failed(snafu::Report::from_error(error).to_string())
            })
    }

    async fn close_notification(&self, id: u32) -> fdo::Result<()> {
        let (reply, response) = oneshot::channel();
        self.0
            .send(Command::Close(id, 3, reply))
            .await
            .map_err(|error| fdo::Error::Failed(error.to_string()))?;
        response
            .await
            .map_err(|error| fdo::Error::Failed(error.to_string()))?
            .map_err(|error| {
                fdo::Error::Failed(snafu::Report::from_error(error).to_string())
            })
    }

    #[zbus(signal)]
    pub(super) async fn notification_closed(
        emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;
}
