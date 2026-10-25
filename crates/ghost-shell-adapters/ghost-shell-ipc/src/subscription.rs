use std::sync::{Arc, Mutex};

use iced::{
    Subscription,
    futures::{SinkExt as _, Stream},
};

use crate::{AsyncRequest, Endpoint, RequestServer as _, Server};

#[derive(Clone)]
pub struct Incoming(Arc<Mutex<Option<AsyncRequest>>>);

impl Incoming {
    pub fn take(&self) -> Option<AsyncRequest> {
        match self.0.lock() {
            Ok(mut request) => request.take(),
            Err(error) => {
                log::error!("IPC request lock: {error}");
                None
            }
        }
    }
}

#[derive(Clone)]
pub enum Event {
    Request(Incoming),
    Failed(String),
}

pub fn listen(endpoint: Endpoint) -> Subscription<Event> {
    Subscription::run_with(endpoint, run)
}

fn run(endpoint: &Endpoint) -> impl Stream<Item = Event> + use<> {
    let endpoint = *endpoint;
    iced::stream::channel(64, async move |mut output| {
        let result: anyhow::Result<()> = async {
            let (sender, mut receiver) = tokio::sync::mpsc::channel(64);
            let server = Server::bind(crate::socket_path(endpoint)?, sender).await?;
            let server = server.run();
            tokio::pin!(server);
            loop {
                tokio::select! {
                    result = &mut server => return result,
                    request = receiver.recv() => {
                        let Some(request) = request else { return Ok(()); };
                        output.send(Event::Request(Incoming(Arc::new(Mutex::new(Some(request)))))).await?;
                    }
                }
            }
        }.await;
        if let Err(error) = result {
            if output
                .send(Event::Failed(format!("{error:#}")))
                .await
                .is_err()
            {
                log::debug!("IPC application receiver closed");
            }
        }
    })
}

pub fn shutdown<Message: Send + 'static>(request: AsyncRequest) -> iced::Task<Message> {
    iced::Task::perform(
        request.respond_and_flush(Ok(crate::Response::Handled)),
        |()| (),
    )
    .then(|()| iced::exit())
}
