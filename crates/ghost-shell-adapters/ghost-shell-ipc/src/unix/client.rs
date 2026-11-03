use std::path::PathBuf;

use snafu::ResultExt as _;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{
        UnixSocket,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
};

use crate::{
    ProtocolSnafu, Result, SocketSnafu, TransportSnafu, UnexpectedDisconnectSnafu,
    protocol::{Reply, Request},
};

pub struct Client {
    reader: BufReader<OwnedReadHalf>,
    writer: BufWriter<OwnedWriteHalf>,
}

impl crate::RequestClient for Client {
    async fn connect(path: PathBuf) -> Result<Self> {
        let socket = UnixSocket::new_stream().context(SocketSnafu {
            operation: "create",
            path: &path,
        })?;
        let stream = socket
            .connect(&path)
            .await
            .context(SocketSnafu {
                operation: "connect to",
                path: &path,
            })?;

        let (reader, writer) = stream.into_split();
        let reader = BufReader::new(reader);
        let writer = BufWriter::new(writer);

        Ok(Self { reader, writer })
    }

    async fn read(&mut self) -> Result<Reply> {
        let mut buf = String::new();
        let count = self
            .reader
            .read_line(&mut buf)
            .await
            .context(TransportSnafu { operation: "read" })?;
        snafu::ensure!(count != 0, UnexpectedDisconnectSnafu);
        let request = serde_json::from_str(&buf).context(ProtocolSnafu {
            operation: "decode reply",
        })?;
        Ok(request)
    }

    async fn write(&mut self, reply: Request) -> Result<()> {
        let mut buf = serde_json::to_string(&reply).context(ProtocolSnafu {
            operation: "encode request",
        })?;
        buf.push('\n');
        self.writer
            .write_all(buf.as_bytes())
            .await
            .context(TransportSnafu {
                operation: "write request",
            })?;
        self.writer
            .flush()
            .await
            .context(TransportSnafu {
                operation: "flush request",
            })?;
        Ok(())
    }
}
