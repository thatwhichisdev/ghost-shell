use anyhow::{Context as _, Result, anyhow};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{UnixListener, UnixStream, unix::SocketAddr},
    sync::{mpsc, oneshot},
    task::JoinSet,
};

use crate::protocol::{Reply, Request};

pub struct AsyncRequest {
    pub request: Request,
    pub reply: oneshot::Sender<Reply>,
}

pub struct Server {
    #[allow(unused)]
    path: PathBuf,
    listener: UnixListener,
    request_sender: mpsc::Sender<AsyncRequest>,
}

struct Connection {
    #[allow(unused)]
    address: SocketAddr,
    stream: UnixStream,
    request_sender: mpsc::Sender<AsyncRequest>,
}

impl Server {
    pub fn bind(
        path: impl AsRef<Path>,
        request_sender: mpsc::Sender<AsyncRequest>,
    ) -> Result<Self> {
        let path = path.as_ref().to_owned();

        if path.exists() {
            fs::remove_file(&path)?;
        }

        let listener = UnixListener::bind(&path)?;

        Ok(Self {
            path,
            listener,
            request_sender,
        })
    }

    pub async fn run(self) -> Result<()> {
        let mut connections = JoinSet::<Result<()>>::new();

        loop {
            tokio::select! {
                accepted = self.listener.accept() => {
                    let (stream, address) = accepted
                        .context("failed to accept IPC connection")?;

                    let connection = Connection::new(
                        address,
                        stream,
                        self.request_sender.clone()
                    );

                    connections.spawn(connection.handle());
                }
                completed = connections.join_next(), if !connections.is_empty() =>
                {
                    match completed {
                        Some(Ok(Ok(()))) => {
                            println!("IPC connection closed normally");
                        }
                        Some(Ok(Err(err))) => {
                            eprintln!("IPC connection failed: {err:#}");
                        }
                        Some(Err(err)) if err.is_panic() => {
                            eprintln!("IPC connection task panicked:{err}");
                        }
                        Some(Err(err)) => {
                            eprintln!("IPC connection task was cancelled: {err}");
                        }
                        None => unreachable!("IPC connection list was not empty"),
                    }
                }

            }
        }
    }
}

impl Connection {
    #[must_use]
    pub fn new(
        address: SocketAddr,
        stream: UnixStream,
        request_sender: mpsc::Sender<AsyncRequest>,
    ) -> Self {
        Self {
            address,
            stream,
            request_sender,
        }
    }

    pub async fn handle(self) -> Result<()> {
        let Self {
            address,
            stream,
            request_sender,
        } = self;

        let (reader, writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut writer = BufWriter::new(writer);
        let mut buffer = String::new();

        loop {
            buffer.clear();

            let bytes_read =
                reader.read_line(&mut buffer).await.with_context(|| {
                    format!("failed to read IPC request from {address:?}")
                })?;

            if bytes_read == 0 {
                // Normal client disconnect.
                return Ok(());
            }

            let request = serde_json::from_str::<Request>(buffer.trim_end())
                .with_context(|| {
                    format!(
                        "failed to deserialize IPC request from \
                     {address:?}; contents: {buffer:?}"
                    )
                })?;

            let (reply_sender, reply_receiver) = oneshot::channel::<Reply>();

            request_sender
                .send(AsyncRequest {
                    request,
                    reply: reply_sender,
                })
                .await
                .map_err(|_| {
                    anyhow!("daemon IPC request receiver was dropped")
                })?;

            let reply = reply_receiver
                .await
                .context("daemon dropped the IPC reply channel")?;

            let mut response = serde_json::to_vec(&reply)
                .context("failed to serialize IPC reply")?;

            response.push(b'\n');

            writer
                .write_all(&response)
                .await
                .context("failed to write IPC reply")?;

            writer.flush().await.context("failed to flush IPC reply")?;
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Err(err) = std::fs::remove_file(&self.path) {
            eprintln!("failed to remove IPC socket {err:#}");
        }
    }
}
