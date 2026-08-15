pub mod client;
pub mod protocol;
pub mod stream;

pub use client::*;
pub use protocol::*;
pub use stream::*;

use gpui::App;
use tokio::sync::mpsc;

pub fn init(cx: &mut App) {
    let tokio = gpui_tokio::Tokio::handle(cx);

    let mut niri_client = tokio.block_on(NiriClient::try_new()).unwrap();
    let mut niri_stream = tokio.block_on(NiriStream::try_new()).unwrap();
    let mut niri_state = NiriState::default();

    // Fetch niri workspaces using niri client
    let niri_workspaces = {
        let response = tokio
            .block_on(niri_client.send(Request::Workspaces))
            .unwrap()
            .unwrap();

        let workspaces = match response {
            Response::Workspaces(workspaces) => workspaces,
            response => {
                panic!("unexpected Niri response: {response:?}");
            }
        };

        workspaces
    };

    // Fetch niri windows using niri client
    let niri_windows = {
        let response = tokio
            .block_on(niri_client.send(Request::Windows))
            .unwrap()
            .unwrap();

        match response {
            Response::Windows(windows) => windows,
            response => {
                panic!("unexpected Niri response: {response:?}");
            }
        }
    };

    // Update niri state with workspaces
    niri_state.update(Event::WorkspacesChanged {
        workspaces: niri_workspaces,
    });

    // Update niri state with windows
    niri_state.update(Event::WindowsChanged {
        windows: niri_windows,
    });

    cx.set_global(niri_client);
    cx.set_global(niri_state);

    // Async primitive to brigde together niri_stream and niri_state
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
