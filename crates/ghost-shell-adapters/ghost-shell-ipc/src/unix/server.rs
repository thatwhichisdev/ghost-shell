use std::{fs, path::PathBuf};

use snafu::ResultExt as _;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{UnixListener, UnixStream, unix::SocketAddr},
    sync::{mpsc, oneshot},
    task::JoinSet,
};

use crate::{
    AlreadyRunningSnafu, AsyncRequest, NotSocketSnafu, ProtocolSnafu, ReplyClosedSnafu,
    RequestReceiverClosedSnafu, Result, SocketSnafu, TransportSnafu,
    protocol::{Reply, Request},
};

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

impl crate::RequestServer for Server {
    async fn bind(
        path: PathBuf,
        request_sender: mpsc::Sender<AsyncRequest>,
    ) -> Result<Self> {
        if path.exists() {
            use std::os::unix::fs::FileTypeExt as _;
            snafu::ensure!(
                fs::symlink_metadata(&path)
                    .context(SocketSnafu {
                        operation: "inspect",
                        path: &path
                    })?
                    .file_type()
                    .is_socket(),
                NotSocketSnafu { path: &path }
            );
            match UnixStream::connect(&path).await {
                Ok(_) => return AlreadyRunningSnafu { path: &path }.fail(),
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::ConnectionRefused
                            | std::io::ErrorKind::NotFound
                    ) =>
                {
                    fs::remove_file(&path).context(SocketSnafu {
                        operation: "remove stale",
                        path: &path,
                    })?
                }
                Err(source) => {
                    return Err(crate::Error::Socket {
                        operation: "probe",
                        path,
                        source,
                    });
                }
            }
        }

        let listener = UnixListener::bind(&path).context(SocketSnafu {
            operation: "bind",
            path: &path,
        })?;

        Ok(Self {
            path,
            listener,
            request_sender,
        })
    }

    async fn run(self) -> Result<()> {
        let mut connections = JoinSet::<Result<()>>::new();

        loop {
            tokio::select! {
                accepted = self.listener.accept() => {
                    let (stream, address) = accepted
                        .context(SocketSnafu { operation: "accept connection on", path: &self.path })?;

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
                            log::debug!("IPC connection closed normally");
                        }
                        Some(Ok(Err(err))) => {
                            log::warn!("IPC connection failed: {}", snafu::Report::from_error(err));
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
            address: _,
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
                reader
                    .read_line(&mut buffer)
                    .await
                    .context(TransportSnafu {
                        operation: "read request",
                    })?;

            if bytes_read == 0 {
                // Normal client disconnect.
                return Ok(());
            }

            let request = serde_json::from_str::<Request>(buffer.trim_end()).context(
                ProtocolSnafu {
                    operation: "decode request",
                },
            )?;

            let (reply_sender, reply_receiver) = oneshot::channel::<Reply>();
            let (delivered_sender, delivered) = oneshot::channel();

            request_sender
                .send(AsyncRequest {
                    request,
                    reply: reply_sender,
                    delivered,
                })
                .await
                .map_err(|_| RequestReceiverClosedSnafu.build())?;

            let reply = reply_receiver
                .await
                .context(ReplyClosedSnafu)?;

            let mut response = serde_json::to_vec(&reply).context(ProtocolSnafu {
                operation: "encode reply",
            })?;

            response.push(b'\n');

            writer
                .write_all(&response)
                .await
                .context(TransportSnafu {
                    operation: "write reply",
                })?;

            writer
                .flush()
                .await
                .context(TransportSnafu {
                    operation: "flush reply",
                })?;
            if delivered_sender.send(()).is_err() {
                log::trace!("IPC caller does not need a flush acknowledgement");
            }
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
