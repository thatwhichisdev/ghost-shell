mod auth;
use ghost_shell_wallpaper::WallpaperSource;
use iced::{
    Element, Fill, Task,
    widget::{column, container, operation, stack, text, text_input},
};

#[derive(Clone)]
pub enum Message {
    Password(String),
    Authenticate,
    Authenticated(u64, Result<(), String>),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Password(_) => "Password([redacted])",
            Self::Authenticate => "Authenticate",
            Self::Authenticated(..) => "Authenticated",
        })
    }
}

#[derive(Default)]
pub struct Lockscreen {
    password: String,
    pub authenticating: bool,
    attempt: u64,
    pub error: Option<String>,
}

impl Lockscreen {
    pub fn reset(&mut self) {
        self.password.clear();
        self.authenticating = false;
        self.error = None;
        self.attempt = self.attempt.wrapping_add(1);
    }

    pub fn accepts(&self, attempt: u64) -> bool {
        self.authenticating && attempt == self.attempt
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Password(password) if !self.authenticating => {
                self.password = password
            }
            Message::Authenticate
                if !self.authenticating && !self.password.is_empty() =>
            {
                let username = auth::username();
                if username.is_empty() {
                    self.error = Some("The session user is unavailable".into());
                    return Task::none();
                }
                self.authenticating = true;
                self.error = None;
                self.attempt = self.attempt.wrapping_add(1);
                let attempt = self.attempt;
                let password = std::mem::take(&mut self.password);
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            auth::authenticate(&username, std::ffi::OsStr::new(&password))
                                .map_err(|_| "Authentication failed".to_owned())
                        })
                        .await
                        .unwrap_or_else(
                            |_| Err("Authentication worker failed".to_owned()),
                        )
                    },
                    move |result| Message::Authenticated(attempt, result),
                );
            }
            Message::Authenticated(attempt, result) if self.accepts(attempt) => {
                self.authenticating = false;
                self.error = result.err();
                return operation::focus("lock-password");
            }
            _ => {}
        }
        Task::none()
    }

    pub fn view<'a>(
        &'a self,
        wallpaper: &'a WallpaperSource,
        clock: &'a str,
    ) -> Element<'a, Message> {
        let input = text_input("Password", &self.password)
            .secure(true)
            .id("lock-password")
            .padding(12)
            .width(280);
        let input = if self.authenticating {
            input
        } else {
            input
                .on_input(Message::Password)
                .on_submit(Message::Authenticate)
        };
        let mut content = column![text(clock).size(128), input]
            .spacing(32)
            .align_x(iced::Center);
        if self.authenticating {
            content = content.push(text("Authenticating…"));
        }
        if let Some(error) = &self.error {
            content = content.push(text(error));
        }
        stack![
            wallpaper.view(),
            container(content)
                .center_x(Fill)
                .center_y(Fill)
        ]
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_rejects_pending_authentication_and_clears_password() {
        let mut state = Lockscreen {
            password: "private".into(),
            authenticating: true,
            attempt: 7,
            error: None,
        };
        assert!(state.accepts(7));
        state.reset();
        assert!(!state.accepts(7));
        assert!(state.password.is_empty());
        drop(state.update(Message::Authenticated(7, Err("stale".into()))));
        assert!(state.error.is_none());
        assert!(
            !format!("{:?}", Message::Password("private".into())).contains("private")
        );
    }

    #[test]
    fn failed_current_attempt_allows_retry() {
        let mut state = Lockscreen {
            authenticating: true,
            attempt: 4,
            ..Default::default()
        };
        drop(state.update(Message::Authenticated(
            4,
            Err("Authentication failed".into()),
        )));
        assert!(!state.authenticating);
        assert_eq!(state.error.as_deref(), Some("Authentication failed"));
        drop(state.update(Message::Password("retry".into())));
        assert_eq!(state.password, "retry");
    }
}
