use std::{env, path::PathBuf, time::Duration};

use iced::task::{Never, Straw, sipper};
use snafu::{OptionExt as _, ResultExt as _, Snafu};
use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader, Lines},
    net::UnixStream,
    time::{error::Elapsed, timeout},
};

use crate::{NiriEvent, Reply, Request as NiriRequest, Response as NiriResponse};

#[derive(Debug, Snafu)]
pub enum StreamError {
    #[snafu(display("NIRI_SOCKET is not set; Ghost Shell must start in a Niri session"))]
    SocketNotConfigured,
    #[snafu(display("Timed out connecting to Niri socket {}: {source}", path.display()))]
    ConnectTimeout { path: PathBuf, source: Elapsed },
    #[snafu(display("Could not connect to Niri socket {}: {source}", path.display()))]
    Connect {
        path: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Could not encode the Niri EventStream request: {source}"))]
    EncodeRequest { source: serde_json::Error },
    #[snafu(display("Could not send the Niri EventStream request: {source}"))]
    SendRequest { source: std::io::Error },
    #[snafu(display("Could not close the Niri stream's write side: {source}"))]
    ShutdownWrite { source: std::io::Error },
    #[snafu(display(
        "Timed out waiting for Niri's EventStream acknowledgement: {source}"
    ))]
    HandshakeTimeout { source: Elapsed },
    #[snafu(display("Could not read Niri's EventStream acknowledgement: {source}"))]
    ReadHandshake { source: std::io::Error },
    #[snafu(display("Niri closed the socket before acknowledging EventStream"))]
    HandshakeClosed,
    #[snafu(display(
        "Invalid Niri EventStream acknowledgement (possible protocol version mismatch): {source}"
    ))]
    DecodeHandshake { source: serde_json::Error },
    #[snafu(display("Niri rejected the EventStream request: {message}"))]
    Rejected { message: String },
    #[snafu(display("Unexpected Niri EventStream acknowledgement: {response:?}"))]
    UnexpectedReply { response: NiriResponse },
    #[snafu(display("Could not read a Niri event: {source}"))]
    ReadEvent { source: std::io::Error },
    #[snafu(display("Niri disconnected"))]
    Disconnected,
    #[snafu(display(
        "Ignoring invalid Niri event (possible Niri/Ghost Shell protocol version mismatch): {source}"
    ))]
    DecodeEvent { source: serde_json::Error },
}

pub struct NiriStream {
    lines: Lines<BufReader<UnixStream>>,
}

impl NiriStream {
    pub async fn try_new() -> Result<Self, StreamError> {
        let socket_path = env::var_os("NIRI_SOCKET")
            .map(PathBuf::from)
            .context(SocketNotConfiguredSnafu)?;

        let mut socket =
            timeout(Duration::from_secs(5), UnixStream::connect(&socket_path))
                .await
                .context(ConnectTimeoutSnafu { path: &socket_path })?
                .context(ConnectSnafu { path: &socket_path })?;

        let mut request =
            serde_json::to_vec(&NiriRequest::EventStream).context(EncodeRequestSnafu)?;

        request.push(b'\n');

        socket
            .write_all(&request)
            .await
            .context(SendRequestSnafu)?;

        socket
            .shutdown()
            .await
            .context(ShutdownWriteSnafu)?;

        let mut lines = BufReader::new(socket).lines();
        let reply = timeout(Duration::from_secs(5), lines.next_line())
            .await
            .context(HandshakeTimeoutSnafu)?
            .context(ReadHandshakeSnafu)?
            .context(HandshakeClosedSnafu)?;

        match serde_json::from_str::<Reply>(&reply).context(DecodeHandshakeSnafu)? {
            Ok(NiriResponse::Handled) => Ok(Self { lines }),
            Ok(response) => UnexpectedReplySnafu { response }.fail(),
            Err(message) => RejectedSnafu { message }.fail(),
        }
    }
}

pub fn connect() -> impl Straw<Never, NiriEvent, StreamError> {
    sipper(async |mut output| {
        let mut stream = NiriStream::try_new().await?;
        loop {
            let line = stream
                .lines
                .next_line()
                .await
                .context(ReadEventSnafu)?
                .context(DisconnectedSnafu)?;

            match serde_json::from_str::<NiriEvent>(&line).context(DecodeEventSnafu) {
                Ok(event) => output.send(event).await,
                Err(error) => log::warn!("{error}"),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touchpad_events_do_not_interrupt_the_event_stream()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()?;
        runtime.block_on(async {
            let (client, mut server) = UnixStream::pair()?;
            let mut stream = NiriStream {
                lines: BufReader::new(client).lines(),
            };
            server
                .write_all(
                    concat!(
                        "{\"TouchpadToggled\":{\"on\":true}}\n",
                        "{\"TouchpadToggled\":{\"on\":false}}\n",
                        "{\"OverviewOpenedOrClosed\":{\"is_open\":true}}\n"
                    )
                    .as_bytes(),
                )
                .await?;
            server.shutdown().await?;

            let mut state = crate::NiriState::default();
            for expected in [true, false] {
                let line = stream
                    .lines
                    .next_line()
                    .await?
                    .ok_or("missing touchpad event")?;
                let event = serde_json::from_str::<NiriEvent>(&line)?;
                assert!(
                    matches!(event, NiriEvent::TouchpadToggled { on } if on == expected)
                );
                state.update(event);
            }
            let line = stream
                .lines
                .next_line()
                .await?
                .ok_or("missing subsequent event")?;
            let event = serde_json::from_str::<NiriEvent>(&line)?;
            assert!(matches!(
                event,
                NiriEvent::OverviewOpenedOrClosed { is_open: true }
            ));
            state.update(event);
            assert!(state.is_open);
            assert!(stream.lines.next_line().await?.is_none());
            Ok(())
        })
    }

    #[test]
    fn touchpad_event_requires_a_boolean_state() {
        assert!(
            serde_json::from_str::<NiriEvent>(r#"{"TouchpadToggled":{"on":"yes"}}"#)
                .is_err()
        );
        assert!(serde_json::from_str::<NiriEvent>(r#"{"TouchpadToggled":{}}"#).is_err());
    }
}

pub fn events() -> impl iced::futures::Stream<
    Item = Result<crate::NiriEvent, std::sync::Arc<crate::StreamError>>,
> {
    use iced::{futures::SinkExt as _, task::Sipper as _};
    iced::stream::channel(128, async |mut output| {
        loop {
            // Await the final result as well as forwarding progress; a plain
            // Stream subscription would miss Straw's terminal error.
            let error = match crate::connect().with(Ok).run(&output).await {
                Ok(never) => match never {},
                Err(error) => error,
            };
            if output
                .send(Err(std::sync::Arc::new(error)))
                .await
                .is_err()
            {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    })
}
