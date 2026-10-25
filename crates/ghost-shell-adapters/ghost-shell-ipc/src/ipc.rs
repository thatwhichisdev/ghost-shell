mod protocol;
pub mod unix;

use std::{future::Future, path::PathBuf};

use anyhow::Result;
pub use protocol::*;
use tokio::sync::{mpsc, oneshot};

pub type Client = unix::Client;
pub type Server = unix::Server;

pub struct AsyncRequest {
    pub request: Request,
    pub reply: oneshot::Sender<Reply>,
    pub(crate) delivered: oneshot::Receiver<()>,
}

pub trait RequestClient: Sized {
    fn connect(path: PathBuf) -> impl Future<Output = Result<Self>> + Send;
    fn read(&mut self) -> impl Future<Output = Result<Reply>> + Send;
    fn write(&mut self, request: Request) -> impl Future<Output = Result<()>> + Send;
}

pub trait RequestServer: Sized {
    fn bind(
        path: PathBuf,
        requests: mpsc::Sender<AsyncRequest>,
    ) -> impl Future<Output = Result<Self>> + Send;
    /// Runs until cancellation or a listener failure. Dropping it stops connections.
    fn run(self) -> impl Future<Output = Result<()>> + Send;
}

#[cfg(feature = "iced")]
pub mod subscription;

pub fn socket_path(endpoint: Endpoint) -> Result<PathBuf> {
    let directory = std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| anyhow::anyhow!("XDG_RUNTIME_DIR is unset"))?;
    Ok(PathBuf::from(directory).join(endpoint.name()))
}

pub async fn call(endpoint: Endpoint, request: Request) -> Result<Reply> {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let mut client = Client::connect(socket_path(endpoint)?).await?;
        client.write(request).await?;
        client.read().await
    })
    .await?
}

impl AsyncRequest {
    pub fn respond(self, reply: Reply) {
        if self.reply.send(reply).is_err() {
            log::debug!("IPC requester disconnected before its reply");
        }
    }
}

impl AsyncRequest {
    pub async fn respond_and_flush(self, reply: Reply) {
        if self.reply.send(reply).is_err() {
            log::debug!("IPC requester disconnected before its reply");
            return;
        }
        if self.delivered.await.is_err() {
            log::debug!("IPC connection closed before flushing its reply");
        }
    }
}
