use anyhow::{Context as _, Result};
use gpui::Global;
use std::{env, path::PathBuf};

use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
};

use crate::protocol::{Reply, Request};

pub struct NiriClient {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl NiriClient {
    #[must_use]
    pub async fn try_new() -> Result<Self> {
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

    async fn write(&mut self, request: Request) -> Result<()> {
        let mut buf = serde_json::to_string(&request)?;
        buf.push('\n');

        self.writer.write_all(buf.as_bytes()).await?;

        Ok(())
    }

    async fn read(&mut self) -> Result<Reply> {
        let mut buf = String::new();
        self.reader.read_line(&mut buf).await?;

        let reply = serde_json::from_str(&buf)?;

        Ok(reply)
    }

    pub async fn send(&mut self, request: Request) -> Result<Reply> {
        self.write(request).await?;
        let reply = self.read().await?;

        Ok(reply)
    }
}

impl Global for NiriClient {}
