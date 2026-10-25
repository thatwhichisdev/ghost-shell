use std::{fmt, sync::Arc};

use greetd_ipc::{AuthMessageType, ErrorType, Request, Response, codec::TokioCodec};
use tokio::{net::UnixStream, sync::Mutex};

#[derive(Clone)]
pub struct Client {
    stream: Arc<Mutex<Option<UnixStream>>>,
}

#[derive(Clone)]
pub enum Reply {
    Success,
    Prompt { message: String, secret: bool },
    Notice(String),
    Rejected { authentication: bool },
}

impl fmt::Debug for Reply {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Success => "Success",
            Self::Prompt { .. } => "Prompt([redacted])",
            Self::Notice(_) => "Notice([redacted])",
            Self::Rejected { .. } => "Rejected",
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Error {
    MissingSocket,
    Connect,
    Disconnected,
    Transport,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingSocket => {
                "GREETD_SOCK is missing; start the greeter through greetd"
            }
            Self::Connect => "Could not connect to greetd",
            Self::Disconnected => {
                "The greetd connection is no longer usable; restart the greeter"
            }
            Self::Transport => "Communication with greetd failed; restart the greeter",
        })
    }
}

impl std::error::Error for Error {}

impl Client {
    pub async fn connect() -> Result<Self, Error> {
        let path = std::env::var_os("GREETD_SOCK")
            .filter(|path| !path.is_empty())
            .ok_or(Error::MissingSocket)?;
        let stream = UnixStream::connect(path)
            .await
            .map_err(|_| Error::Connect)?;
        Ok(Self {
            stream: Arc::new(Mutex::new(Some(stream))),
        })
    }

    pub async fn create_session(&self, username: String) -> Result<Reply, Error> {
        self.exchange(Request::CreateSession { username })
            .await
    }

    pub async fn respond(&self, response: Option<String>) -> Result<Reply, Error> {
        self.exchange(Request::PostAuthMessageResponse { response })
            .await
    }

    pub async fn start_session(
        &self,
        command: Vec<String>,
        environment: Vec<String>,
    ) -> Result<Reply, Error> {
        self.exchange(Request::StartSession {
            cmd: command,
            env: environment,
        })
        .await
    }

    pub async fn cancel_session(&self) -> Result<Reply, Error> {
        self.exchange(Request::CancelSession).await
    }

    async fn exchange(&self, request: Request) -> Result<Reply, Error> {
        let mut connection = self.stream.lock().await;
        // Cancellation or a partial exchange must never leave a stream available
        // for another request: its next reply could belong to the previous one.
        let mut stream = connection
            .take()
            .ok_or(Error::Disconnected)?;
        request
            .write_to(&mut stream)
            .await
            .map_err(|_| Error::Transport)?;
        drop(request);
        let response = Response::read_from(&mut stream)
            .await
            .map_err(|_| Error::Transport)?;
        *connection = Some(stream);
        Ok(match response {
            Response::Success => Reply::Success,
            Response::AuthMessage {
                auth_message_type,
                auth_message,
            } => match auth_message_type {
                AuthMessageType::Secret => Reply::Prompt {
                    message: auth_message,
                    secret: true,
                },
                AuthMessageType::Visible => Reply::Prompt {
                    message: auth_message,
                    secret: false,
                },
                AuthMessageType::Info | AuthMessageType::Error => {
                    Reply::Notice(auth_message)
                }
            },
            // Server descriptions can include credentials; do not expose them
            // through errors, Debug output, or logs.
            Response::Error { error_type, .. } => Reply::Rejected {
                authentication: matches!(error_type, ErrorType::AuthError),
            },
        })
    }
}
