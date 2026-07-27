use anyhow::{Context as _, Result};
use std::{env, path::PathBuf};

use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::broadcast::{self, Receiver, Sender},
};

use crate::client::{
    event::Event, request::Request, response::Reply, state::NiriState,
};

pub struct NiriClient {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl NiriClient {
    #[must_use]
    pub async fn new() -> Result<Self> {
        let socket_path = env::var_os("NIRI_SOCKET")
            .map(PathBuf::from)
            .context("NIRI_SOCKET is not set; is Ghost running under Niri?")?;

        let stream =
            UnixStream::connect(&socket_path).await.with_context(|| {
                format!(
                    "failed to connect to Niri socket {}",
                    socket_path.display()
                )
            })?;

        let (reader, writer) = stream.into_split();

        Ok(Self {
            reader: BufReader::new(reader),
            writer,
        })
    }

    pub async fn send(&mut self, request: Request) -> Result<Reply> {
        let mut buf = serde_json::to_string(&request).unwrap();
        buf.push('\n');

        self.writer.write_all(buf.as_bytes()).await.unwrap();

        buf.clear();
        self.reader.read_line(&mut buf).await.unwrap();

        let reply = serde_json::from_str(&buf).unwrap();
        Ok(reply)
    }

    pub async fn into_event_reader(mut self) -> NiriStateReader {
        let _ = self.send(Request::EventStream).await.unwrap();
        self.writer.shutdown().await.unwrap();

        NiriStateReader::new(self.reader)
    }
}

pub struct NiriStateReader {
    reader: BufReader<OwnedReadHalf>,
    sender: Sender<NiriState>,
    state: NiriState,
}

impl NiriStateReader {
    #[must_use]
    pub fn new(reader: BufReader<OwnedReadHalf>) -> Self {
        let (sender, _) = broadcast::channel(256);
        Self {
            reader,
            sender,
            state: NiriState::default(),
        }
    }

    pub fn subscribe(&self) -> Receiver<NiriState> {
        self.sender.subscribe()
    }

    pub async fn read_events(&mut self) -> Result<Event> {
        let mut buf = String::new();
        self.reader.read_line(&mut buf).await.unwrap();

        serde_json::from_str(&buf).map_err(|err| err.into())
    }

    pub async fn run(mut self) {
        loop {
            match self.read_events().await {
                Ok(event) => {
                    self.state.update(event);
                    self.sender.send(self.state.clone()).unwrap();
                }
                Err(err) => {
                    eprintln!("failed to process niri event {err:?}");
                }
            }
        }
    }
}
