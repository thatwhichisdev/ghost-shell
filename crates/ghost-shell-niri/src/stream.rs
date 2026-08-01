use anyhow::{Context as _, Result};
use std::{env, path::PathBuf};

use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
};

use crate::{Event, Reply, Request};

pub struct NiriStream {
    reader: BufReader<OwnedReadHalf>,

    #[allow(unused)]
    writer: OwnedWriteHalf,
}

impl NiriStream {
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

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        let mut buf = serde_json::to_string(&Request::EventStream)?;
        buf.push('\n');

        writer.write_all(buf.as_bytes()).await?;

        buf.clear();
        reader.read_line(&mut buf).await?;

        match serde_json::from_str(&buf) {
            Ok(Reply::Ok(crate::Response::Handled)) => {
                writer.shutdown().await?;

                Ok(Self { reader, writer })
            }
            Ok(Reply::Ok(_)) => {
                Err(anyhow::Error::msg("wrong response from niri"))
            }
            Ok(Err(err)) => Err(anyhow::Error::msg(err)),
            Err(err) => Err(anyhow::Error::msg(err.to_string())),
        }
    }

    pub async fn read(&mut self) -> Result<Event> {
        let mut buf = String::new();
        self.reader.read_line(&mut buf).await?;

        serde_json::from_str(&buf).map_err(Into::into)
    }
}
