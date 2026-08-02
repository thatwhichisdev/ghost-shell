pub mod client;
pub mod protocol;
pub mod stream;

pub use client::*;
pub use protocol::*;
pub use stream::*;

use gpui::App;
use tokio::sync::mpsc;

pub fn init(cx: &mut App) {
    let niri_client = gpui_tokio::Tokio::handle(cx)
        .block_on(NiriClient::try_new())
        .unwrap();

    let mut niri_stream = gpui_tokio::Tokio::handle(cx)
        .block_on(NiriStream::try_new())
        .unwrap();

    let niri_state = NiriState::default();

    cx.set_global(niri_client);
    cx.set_global(niri_state);

    let (event_sender, mut event_receiver) = mpsc::channel(256);

    // Spawn niri stream reader task on tokio runtime
    gpui_tokio::Tokio::spawn(cx, async move {
        loop {
            match niri_stream.read().await {
                Ok(Some(event)) => {
                    event_sender.send(event).await.unwrap();
                }
                Ok(None) => {
                    eprintln!("Niri event stream closed");
                    break;
                }
                Err(err) => {
                    eprintln!("Failed to read niri event {err:#}");
                    continue;
                }
            }
        }
    })
    .detach();

    // Spawn niri state updating task on gpui runtime
    cx.spawn(async move |cx| {
        while let Some(event) = event_receiver.recv().await {
            cx.update_global::<NiriState, _>(|state, _cx| {
                state.update(event);
            });
        }
    })
    .detach();
}
