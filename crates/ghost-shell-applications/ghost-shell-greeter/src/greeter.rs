use ghost_shell_config::GreeterSessionConfig;
use ghost_shell_greetd::{Client, Error, Reply};
use ghost_shell_wallpaper::WallpaperSource;
use iced::{
    Element, Fill, Task,
    widget::{button, column, container, operation, scrollable, stack, text, text_input},
};

#[derive(Clone)]
pub enum Message {
    Username(String),
    Answer(String),
    Submit,
    Cancel,
    Connected(Result<Client, Error>),
    Reply(Result<Reply, Error>),
    Restart,
}

impl std::fmt::Debug for Message {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("GreeterMessage([redacted])")
    }
}

#[derive(Clone, Copy)]
enum Operation {
    Authenticate,
    Start,
    Cancel,
}

enum Phase {
    Connecting,
    Username,
    Prompt { message: String, secret: bool },
    Working(Operation),
    Fatal,
    Started,
}

pub struct Greeter {
    client: Option<Client>,
    session: GreeterSessionConfig,
    username: String,
    answer: String,
    phase: Phase,
    notice: Option<String>,
    error: Option<String>,
}

impl Greeter {
    pub fn new(session: GreeterSessionConfig) -> (Self, Task<Message>) {
        (
            Self {
                client: None,
                session,
                username: String::new(),
                answer: String::new(),
                phase: Phase::Connecting,
                notice: None,
                error: None,
            },
            Task::perform(Client::connect(), Message::Connected),
        )
    }

    pub fn started(&self) -> bool {
        matches!(self.phase, Phase::Started)
    }

    pub fn focus(&self) -> Task<Message> {
        operation::focus(if matches!(self.phase, Phase::Prompt { .. }) {
            "greeter-answer"
        } else {
            "greeter-username"
        })
    }

    fn fatal(&mut self, error: impl ToString) {
        self.answer.clear();
        self.phase = Phase::Fatal;
        self.notice = None;
        self.error = Some(error.to_string());
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Connected(result) if matches!(self.phase, Phase::Connecting) => {
                match result {
                    Ok(client) => {
                        self.client = Some(client);
                        self.phase = Phase::Username;
                        return self.focus();
                    }
                    Err(error) => self.fatal(error),
                }
            }
            Message::Username(username) if matches!(self.phase, Phase::Username) => {
                self.username = username;
            }
            Message::Answer(answer) if matches!(self.phase, Phase::Prompt { .. }) => {
                self.answer = answer;
            }
            Message::Submit => {
                let Some(client) = self.client.clone() else {
                    return Task::none();
                };
                match self.phase {
                    Phase::Username if !self.username.trim().is_empty() => {
                        self.username = self.username.trim().to_owned();
                        let username = self.username.clone();
                        self.error = None;
                        self.notice = None;
                        self.phase = Phase::Working(Operation::Authenticate);
                        return Task::perform(
                            async move { client.create_session(username).await },
                            Message::Reply,
                        );
                    }
                    Phase::Prompt { .. } => {
                        let answer = std::mem::take(&mut self.answer);
                        self.phase = Phase::Working(Operation::Authenticate);
                        return Task::perform(
                            async move { client.respond(Some(answer)).await },
                            Message::Reply,
                        );
                    }
                    _ => {}
                }
            }
            Message::Cancel if matches!(self.phase, Phase::Prompt { .. }) => {
                self.answer.clear();
                self.notice = None;
                return self.cancel();
            }
            Message::Reply(result) => {
                let Phase::Working(operation) = self.phase else {
                    return Task::none();
                };
                let reply = match result {
                    Ok(reply) => reply,
                    Err(error) => {
                        self.fatal(error);
                        return Task::none();
                    }
                };
                match (operation, reply) {
                    (Operation::Cancel, Reply::Success) => {
                        self.phase = Phase::Username;
                        return self.focus();
                    }
                    (Operation::Cancel, _) => {
                        self.fatal("Could not cancel authentication; restart the greeter")
                    }
                    (Operation::Start, Reply::Success) => self.phase = Phase::Started,
                    (_, Reply::Rejected { authentication }) => {
                        self.notice = None;
                        self.error = Some(if authentication {
                            "Authentication failed. Please try again."
                        } else {
                            "greetd could not complete the request. Check the system journal."
                        }.into());
                        return self.cancel();
                    }
                    (Operation::Authenticate, Reply::Prompt { message, secret }) => {
                        self.answer.clear();
                        self.phase = Phase::Prompt { message, secret };
                        return self.focus();
                    }
                    (Operation::Authenticate, Reply::Notice(message)) => {
                        self.notice = Some(message);
                        if let Some(client) = self.client.clone() {
                            return Task::perform(
                                async move { client.respond(None).await },
                                Message::Reply,
                            );
                        }
                    }
                    (Operation::Authenticate, Reply::Success) => {
                        if let Some(client) = self.client.clone() {
                            self.phase = Phase::Working(Operation::Start);
                            let command = self.session.command.clone();
                            let environment = self.session.environment.clone();
                            return Task::perform(
                                async move {
                                    client
                                        .start_session(command, environment)
                                        .await
                                },
                                Message::Reply,
                            );
                        }
                    }
                    (Operation::Start, _) => self
                        .fatal("Unexpected session-start response; restart the greeter"),
                }
            }
            Message::Restart if matches!(self.phase, Phase::Fatal) => {
                return iced::exit();
            }
            _ => {}
        }
        Task::none()
    }

    fn cancel(&mut self) -> Task<Message> {
        self.answer.clear();
        self.phase = Phase::Working(Operation::Cancel);
        if let Some(client) = self.client.clone() {
            Task::perform(async move { client.cancel_session().await }, Message::Reply)
        } else {
            self.fatal("The greetd connection is unavailable");
            Task::none()
        }
    }

    pub fn view<'a>(
        &'a self,
        wallpaper: &'a WallpaperSource,
        clock: &'a str,
    ) -> Element<'a, Message> {
        let username = text_input("Username", &self.username)
            .id("greeter-username")
            .padding(12);
        let username = if matches!(self.phase, Phase::Username) {
            username
                .on_input(Message::Username)
                .on_submit(Message::Submit)
        } else {
            username
        };
        let mut form = column![username].spacing(12).width(360);
        match &self.phase {
            Phase::Prompt { message, secret } => {
                form = form
                    .push(text(message))
                    .push(
                        text_input(
                            if *secret {
                                "Password or secret"
                            } else {
                                "Response"
                            },
                            &self.answer,
                        )
                        .secure(*secret)
                        .id("greeter-answer")
                        .padding(12)
                        .on_input(Message::Answer)
                        .on_submit(Message::Submit),
                    )
                    .push(button("Continue").on_press(Message::Submit))
                    .push(button("Change user").on_press(Message::Cancel));
            }
            Phase::Username => {
                form = form
                    .push(
                        text_input("Password", "")
                            .secure(true)
                            .padding(12),
                    )
                    .push(button("Log in").on_press_maybe(
                        (!self.username.trim().is_empty()).then_some(Message::Submit),
                    ));
            }
            Phase::Connecting => form = form.push(text("Connecting to greetd…")),
            Phase::Working(operation) => {
                form = form.push(text(match operation {
                    Operation::Authenticate => "Authenticating…",
                    Operation::Start => "Starting session…",
                    Operation::Cancel => "Cancelling authentication…",
                }))
            }
            Phase::Fatal => {
                form = form.push(button("Restart greeter").on_press(Message::Restart))
            }
            Phase::Started => form = form.push(text("Starting session…")),
        }
        if let Some(notice) = &self.notice {
            form = form.push(text(notice));
        }
        if let Some(error) = &self.error {
            form = form.push(text(error));
        }
        let content = column![text(clock).size(128), form]
            .spacing(32)
            .align_x(iced::Center);
        stack![
            wallpaper.view(),
            scrollable(
                container(content)
                    .padding(24)
                    .center_x(Fill)
                    .center_y(Fill)
            )
            .height(Fill)
        ]
        .into()
    }
}
