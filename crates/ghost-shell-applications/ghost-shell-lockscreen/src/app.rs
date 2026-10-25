use std::collections::HashSet;

use ghost_shell_config::LockscreenConfig;
use ghost_shell_ipc::{AsyncRequest, Endpoint, Request, Response, subscription as ipc};
use ghost_shell_lockscreen::{Lockscreen, Message as Input};
use ghost_shell_wallpaper::WallpaperSource;
use iced::{Element, Subscription, Task, widget::operation, window::Id};
use iced_exwlshell::{
    settings::{ExWlSettings, LayerShellSettings, StartMode},
    to_exwlshell_message,
};
use iced_wayland_subscriber::shell::{self, ShellEvent, ShellInfo, ShellType};

#[to_exwlshell_message]
#[derive(Clone)]
enum Message {
    Shell(ShellEvent),
    Created(ShellInfo),
    Ipc(ipc::Event),
    Input(Input),
    Wallpaper(Result<WallpaperSource, String>),
    Theme(iced::theme::Mode),
    Tick,
    Stop(Result<(), String>),
}
impl std::fmt::Debug for Message {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("LockscreenMessage")
    }
}
#[derive(PartialEq, Eq)]
enum State {
    Unlocked,
    Pending,
    Locked,
    Unlocking,
}
struct App {
    config: LockscreenConfig,
    events: shell::ShellReceiver,
    windows: HashSet<Id>,
    lockscreen: Lockscreen,
    state: State,
    wallpaper: WallpaperSource,
    clock: String,
    system_dark: bool,
    pending: Vec<AsyncRequest>,
    exit_after_unlock: bool,
}
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = ghost_shell_config::load::<LockscreenConfig>("lockscreen")
        .unwrap_or_else(|error| {
            log::warn!("Configuration: {error}");
            LockscreenConfig::default()
        });
    let font = iced::Font {
        family: match config.general.font_family.as_str() {
            "monospace" => iced::font::Family::Monospace,
            "sans-serif" => iced::font::Family::SansSerif,
            "serif" => iced::font::Family::Serif,
            _ => iced::font::Family::Name(Box::leak(
                config
                    .general
                    .font_family
                    .clone()
                    .into_boxed_str(),
            )),
        },
        ..Default::default()
    };
    let font_size = config.general.font_size;
    let (shell_broadcast, events) = shell::channel();
    iced_exwlshell::daemon(
        move || App::new(config.clone(), events.clone()),
        "ghost-shell-lockscreen",
        App::update,
        App::view,
    )
    .subscription(App::subscription)
    .theme(|app: &App, _| ghost_shell_theme::theme(&app.config.theme, app.system_dark))
    .default_font(font)
    .default_text_size(font_size)
    .on_new_shell(|info| {
        (info.shell == ShellType::SessionLock).then_some(Message::Created(info))
    })
    .wl_settings(ExWlSettings {
        shell_broadcast,
        layer_settings: LayerShellSettings {
            start_mode: StartMode::Background,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()?;
    Ok(())
}
impl App {
    fn new(
        config: LockscreenConfig,
        events: shell::ShellReceiver,
    ) -> (Self, Task<Message>) {
        let wallpaper =
            WallpaperSource::Solid(ghost_shell_theme::rgba(config.wallpaper.bg));
        let mut tasks = vec![iced::system::theme().map(Message::Theme)];
        if let Some(path) = config.wallpaper.path.clone() {
            tasks.push(Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        WallpaperSource::load(std::path::Path::new(&path))
                            .map_err(|error| error.to_string())
                    })
                    .await
                    .unwrap_or_else(|error| Err(error.to_string()))
                },
                Message::Wallpaper,
            ));
        }
        (
            Self {
                config,
                events,
                windows: HashSet::new(),
                lockscreen: Default::default(),
                state: State::Unlocked,
                wallpaper,
                clock: jiff::Zoned::now()
                    .strftime("%H:%M")
                    .to_string(),
                system_dark: true,
                pending: Vec::new(),
                exit_after_unlock: false,
            },
            Task::batch(tasks),
        )
    }
    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.events.listen().map(Message::Shell),
            Subscription::run(stop_signals).map(Message::Stop),
            ipc::listen(Endpoint::Lockscreen).map(Message::Ipc),
            iced::system::theme_changes().map(Message::Theme),
            if self.state == State::Unlocked {
                Subscription::none()
            } else {
                iced::time::every(std::time::Duration::from_secs(60))
                    .map(|_| Message::Tick)
            },
        ])
    }
    fn unlocked(&mut self) -> Task<Message> {
        self.state = State::Unlocked;
        self.windows.clear();
        self.lockscreen.reset();
        for request in self.pending.drain(..) {
            request.respond(Err("Session lock ended before acceptance".into()));
        }
        if self.exit_after_unlock {
            iced::exit()
        } else {
            Task::none()
        }
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Stop(result) => {
                if let Err(error) = result {
                    log::error!("Shutdown signal listener: {error}");
                }
                self.exit_after_unlock = true;
                if self.state == State::Unlocked {
                    return iced::exit();
                }
            }
            Message::Ipc(ipc::Event::Failed(error)) => {
                log::error!("IPC: {error}");
                if self.state == State::Unlocked {
                    return iced::exit();
                }
                self.exit_after_unlock = true;
            }
            Message::Ipc(ipc::Event::Request(incoming)) => {
                if let Some(request) = incoming.take() {
                    match &request.request {
                        Request::Ping => request.respond(Ok(Response::Handled)),
                        Request::Shutdown => {
                            self.exit_after_unlock = true;
                            if self.state == State::Unlocked {
                                return ipc::shutdown(request);
                            }
                            request.respond(Ok(Response::Handled));
                        }
                        Request::Lock { .. } => match self.state {
                            State::Unlocked => {
                                self.pending.push(request);
                                self.state = State::Pending;
                                self.lockscreen.reset();
                                self.clock = jiff::Zoned::now()
                                    .strftime("%H:%M")
                                    .to_string();
                                return Task::done(Message::Lock);
                            }
                            State::Pending => self.pending.push(request),
                            State::Locked => request.respond(Ok(Response::Handled)),
                            State::Unlocking => request
                                .respond(Err("Session unlock is in progress".into())),
                        },
                        _ => request.respond(Err(
                            "Command is not supported by ghost-shell-lockscreen".into(),
                        )),
                    }
                }
            }
            Message::Created(info) | Message::Shell(ShellEvent::NewShell(info))
                if info.shell == ShellType::SessionLock =>
            {
                self.windows.insert(info.window);
                return operation::focus("lock-password");
            }
            Message::Shell(ShellEvent::Locked) => {
                self.state = State::Locked;
                for request in self.pending.drain(..) {
                    request.respond(Ok(Response::Handled));
                }
            }
            Message::Shell(ShellEvent::LockDenied) => {
                for request in self.pending.drain(..) {
                    request.respond(Err("Compositor denied session lock".into()));
                }
                return self.unlocked();
            }
            Message::Shell(ShellEvent::LockedFinished) => return self.unlocked(),
            Message::Shell(ShellEvent::Closed(window)) => {
                self.windows.remove(&window);
                if self.state == State::Unlocking && self.windows.is_empty() {
                    return self.unlocked();
                }
            }
            Message::Input(Input::Authenticated(attempt, Ok(())))
                if self.state == State::Locked && self.lockscreen.accepts(attempt) =>
            {
                self.state = State::Unlocking;
                self.lockscreen.reset();
                return Task::done(Message::UnLock);
            }
            Message::Input(input) if self.state == State::Locked => {
                return self
                    .lockscreen
                    .update(input)
                    .map(Message::Input);
            }
            Message::Wallpaper(Ok(source)) => self.wallpaper = source,
            Message::Wallpaper(Err(error)) => log::error!("Wallpaper: {error}"),
            Message::Theme(mode) => self.system_dark = mode != iced::theme::Mode::Light,
            Message::Tick => {
                self.clock = jiff::Zoned::now()
                    .strftime("%H:%M")
                    .to_string()
            }
            _ => {}
        }
        Task::none()
    }
    fn view(&self, _window: Id) -> Element<'_, Message> {
        self.lockscreen
            .view(&self.wallpaper, &self.clock)
            .map(Message::Input)
    }
}

fn stop_signals() -> impl iced::futures::Stream<Item = Result<(), String>> {
    use iced::futures::SinkExt as _;
    use tokio::signal::unix::{SignalKind, signal};

    iced::stream::channel(1, async |mut output| {
        let signals = signal(SignalKind::terminate()).and_then(|terminate| {
            signal(SignalKind::interrupt()).map(|interrupt| (terminate, interrupt))
        });
        let (mut terminate, mut interrupt) = match signals {
            Ok(signals) => signals,
            Err(error) => {
                if output
                    .send(Err(error.to_string()))
                    .await
                    .is_err()
                {
                    log::debug!("Lockscreen signal receiver closed");
                }
                return;
            }
        };
        loop {
            tokio::select! {
                _ = terminate.recv() => {},
                _ = interrupt.recv() => {},
            }
            if output.send(Ok(())).await.is_err() {
                return;
            }
        }
    })
}
