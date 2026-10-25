use std::{env, path::PathBuf};

use deadpool::managed;
use iced::Task;
use snafu::{OptionExt as _, ResultExt as _, Snafu};
use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::OnceCell,
};

use crate::{Reply, Request as NiriRequest, Response as NiriResponse};

pub type NiriClientPool = managed::Pool<NiriClientManager>;

static CLIENT_POOL: OnceCell<NiriClientPool> = OnceCell::const_new();

pub struct NiriClientManager;

impl managed::Manager for NiriClientManager {
    type Type = NiriClient;
    type Error = NiriClientError;

    async fn create(&self) -> Result<NiriClient, NiriClientError> {
        NiriClient::try_new().await
    }

    async fn recycle(
        &self,
        client: &mut NiriClient,
        _metrics: &managed::Metrics,
    ) -> managed::RecycleResult<NiriClientError> {
        if client.reusable {
            Ok(())
        } else {
            Err(managed::RecycleError::message(
                "Niri request did not complete cleanly",
            ))
        }
    }
}

pub fn send(request: NiriRequest) -> Task<Result<(), NiriClientError>> {
    Task::perform(
        async move {
            let result = async {
                snafu::ensure!(
                    !matches!(request, NiriRequest::EventStream),
                    EventStreamRequestSnafu
                );
                let pool = CLIENT_POOL
                    .get_or_try_init(|| async {
                        NiriClientPool::builder(NiriClientManager)
                            .max_size(1)
                            .build()
                            .context(PoolBuildSnafu)
                    })
                    .await?;
                let mut client = pool
                    .get()
                    .await
                    .map_err(|error| match error {
                        managed::PoolError::Backend(error) => error,
                        source => NiriClientError::Pool {
                            source: Box::new(source),
                        },
                    })?;
                match client.send(request).await? {
                    Ok(NiriResponse::Handled) => Ok(()),
                    Ok(response) => UnexpectedReplySnafu { response }.fail(),
                    Err(message) => RejectedSnafu { message }.fail(),
                }
            }
            .await;
            if let Err(error) = &result {
                log::error!("Niri request: {error}");
            }
            result
        },
        std::convert::identity,
    )
}

#[derive(Debug, Snafu)]
pub enum NiriClientError {
    #[snafu(display("Could not build the Niri client pool: {source}"))]
    PoolBuild { source: managed::BuildError },
    #[snafu(display("Could not acquire a Niri client: {source}"))]
    Pool {
        source: Box<managed::PoolError<NiriClientError>>,
    },
    #[snafu(display("Use the Niri event stream subscription for EventStream requests"))]
    EventStreamRequest,
    #[snafu(display("Niri rejected the request: {message}"))]
    Rejected { message: String },
    #[snafu(display("Unexpected Niri command reply: {response:?}"))]
    UnexpectedReply { response: NiriResponse },
    #[snafu(display("NIRI_SOCKET is not set; Ghost Shell must start in a Niri session"))]
    SocketNotConfigured,
    #[snafu(display("Could not connect to Niri socket {}: {source}", path.display()))]
    Connect {
        path: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Could not encode a Niri request: {source}"))]
    Encode { source: serde_json::Error },
    #[snafu(display("Could not send a Niri request: {source}"))]
    Write { source: std::io::Error },
    #[snafu(display("Could not read a Niri reply: {source}"))]
    Read { source: std::io::Error },
    #[snafu(display("Niri closed the socket before replying"))]
    Disconnected,
    #[snafu(display(
        "Invalid Niri reply (possible protocol version mismatch): {source}"
    ))]
    Decode { source: serde_json::Error },
}

pub struct NiriClient {
    reusable: bool,
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl NiriClient {
    async fn try_new() -> Result<Self, NiriClientError> {
        let socket_path = env::var_os("NIRI_SOCKET")
            .map(PathBuf::from)
            .context(SocketNotConfiguredSnafu)?;

        let stream = UnixStream::connect(&socket_path)
            .await
            .context(ConnectSnafu { path: socket_path })?;

        let (reader, writer) = stream.into_split();

        Ok(Self {
            reusable: true,
            reader: BufReader::new(reader),
            writer,
        })
    }

    async fn send(&mut self, request: NiriRequest) -> Result<Reply, NiriClientError> {
        let mut buffer = serde_json::to_string(&request).context(EncodeSnafu)?;

        buffer.push('\n');

        // A cancelled or failed exchange may leave an unread reply. Recycling
        // must reject this socket unless a complete reply has been decoded.
        self.reusable = false;

        self.writer
            .write_all(buffer.as_bytes())
            .await
            .context(WriteSnafu)?;

        buffer.clear();

        let length = self
            .reader
            .read_line(&mut buffer)
            .await
            .context(ReadSnafu)?;

        snafu::ensure!(length != 0, DisconnectedSnafu);

        let reply = serde_json::from_str(&buffer).context(DecodeSnafu)?;

        self.reusable = true;

        Ok(reply)
    }
}
