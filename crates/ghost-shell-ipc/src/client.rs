use crate::protocol::{Reply, Request};
use std::path::Path;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{
        UnixSocket,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
};

use anyhow::Result;

pub struct Client {
    reader: BufReader<OwnedReadHalf>,
    writer: BufWriter<OwnedWriteHalf>,
}

impl Client {
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self> {
        let socket = UnixSocket::new_stream()?;
        let stream = socket.connect(path).await?;

        let (reader, writer) = stream.into_split();
        let reader = BufReader::new(reader);
        let writer = BufWriter::new(writer);

        Ok(Self { reader, writer })
    }

    pub async fn read(&mut self) -> Result<Reply> {
        let mut buf = String::new();
        self.reader.read_line(&mut buf).await?;
        let request = serde_json::from_str(&buf)?;
        Ok(request)
    }

    pub async fn write(&mut self, reply: Request) -> Result<()> {
        let mut buf = serde_json::to_string(&reply)?;
        buf.push('\n');
        self.writer.write_all(buf.as_bytes()).await?;
        self.writer.flush().await?;
        Ok(())
    }
}
