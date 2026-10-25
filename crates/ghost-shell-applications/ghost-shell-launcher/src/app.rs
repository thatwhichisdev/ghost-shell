use std::sync::Arc;

use ghost_shell_config::LauncherConfig;
use iced::{
    Element, Fill, Subscription, Task,
    widget::{container, operation, text},
    window::Id,
};
use iced_exwlshell::{
    reexport::{
        Anchor, BlurOption, KeyboardInteractivity, Layer, LayerSize,
        NewLayerShellSettings, OutputOption,
    },
    settings::{ExWlSettings, LayerShellSettings, StartMode},
    to_exwlshell_message,
};
use iced_wayland_subscriber::shell::{self, ShellEvent};

#[to_exwlshell_message]
#[derive(Clone)]
enum Message {
    Shell(ShellEvent),
    Feature(ghost_shell_launcher::Message),
    Key(iced::keyboard::key::Named, bool),
    Theme(iced::theme::Mode),
    LaunchResult(Result<(), Arc<ghost_shell_niri::NiriClientError>>),
}
impl std::fmt::Debug for Message {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("LauncherMessage")
    }
}
struct App {
    config: LauncherConfig,
    events: shell::ShellReceiver,
    window: Id,
    feature: ghost_shell_launcher::Launcher,
    system_dark: bool,
}
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config =
        ghost_shell_config::load::<LauncherConfig>("launcher").unwrap_or_else(|error| {
            log::warn!("Configuration: {error}");
            LauncherConfig::default()
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
        "ghost-shell-launcher",
        App::update,
        App::view,
    )
    .subscription(App::subscription)
    .theme(|app: &App, _| ghost_shell_theme::theme(&app.config.theme, app.system_dark))
    .style(|_, theme| iced::theme::Style {
        background_color: iced::Color::TRANSPARENT,
        text_color: theme.palette().text,
    })
    .default_font(font)
    .default_text_size(font_size)
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
        config: LauncherConfig,
        events: shell::ShellReceiver,
    ) -> (Self, Task<Message>) {
        let app = Self {
            config,
            events,
            window: Id::unique(),
            feature: Default::default(),
            system_dark: true,
        };
        let open = app.open();
        let load = ghost_shell_launcher::Launcher::load().map(Message::Feature);
        (
            app,
            Task::batch([open, load, iced::system::theme().map(Message::Theme)]),
        )
    }
    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.events.listen().map(Message::Shell),
            iced::system::theme_changes().map(Message::Theme),
            iced::event::listen_with(|event, status, _| match event {
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key: iced::keyboard::Key::Named(key),
                    modifiers,
                    ..
                }) if status == iced::event::Status::Ignored
                    || matches!(
                        key,
                        iced::keyboard::key::Named::Escape
                            | iced::keyboard::key::Named::ArrowUp
                            | iced::keyboard::key::Named::ArrowDown
                    ) =>
                {
                    Some(Message::Key(key, modifiers.shift()))
                }
                _ => None,
            }),
        ])
    }
    fn close(&mut self) -> Task<Message> {
        iced::exit()
    }
    fn open(&self) -> Task<Message> {
        Task::done(Message::NewLayerShell {
            id: self.window,
            settings: NewLayerShellSettings {
                layer: Layer::Overlay,
                anchor: Anchor::empty(),
                size: LayerSize::px(540, 450),
                exclusive_zone: Some(-1),
                keyboard_interactivity: KeyboardInteractivity::Exclusive,
                output_option: OutputOption::Active,
                blur_option: if self.config.launcher.blur {
                    BlurOption::FullRegion
                } else {
                    BlurOption::None
                },
                namespace: Some("ghost-shell-launcher".into()),
                ..Default::default()
            },
        })
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Shell(ShellEvent::Closed(window)) if window == self.window => {
                return self.close();
            }
            Message::Shell(ShellEvent::NewShell(info)) if info.window == self.window => {
                return operation::focus("launcher-query");
            }
            Message::Feature(ghost_shell_launcher::Message::Launch(command)) => {
                return ghost_shell_niri::send(ghost_shell_niri::Request::Action(
                    ghost_shell_niri::Action::Spawn { command },
                ))
                .map(|result| Message::LaunchResult(result.map_err(Arc::new)));
            }
            Message::LaunchResult(Ok(())) => return self.close(),
            Message::LaunchResult(Err(error)) => {
                self.feature.error = Some(error.to_string())
            }
            Message::Feature(message) => {
                return self
                    .feature
                    .update(message)
                    .map(Message::Feature);
            }
            Message::Key(key, shift) => {
                use iced::keyboard::key::Named;
                match key {
                    Named::Escape => return self.close(),
                    Named::ArrowUp | Named::ArrowDown => {
                        return self
                            .feature
                            .update(ghost_shell_launcher::Message::Move(
                                key == Named::ArrowDown,
                            ))
                            .map(Message::Feature);
                    }
                    Named::Tab => {
                        return if shift {
                            operation::focus_previous()
                        } else {
                            operation::focus_next()
                        };
                    }
                    _ => {}
                }
            }
            Message::Theme(mode) => self.system_dark = mode != iced::theme::Mode::Light,
            _ => {}
        }
        Task::none()
    }
    fn view(&self, window: Id) -> Element<'_, Message> {
        if self.window != window {
            return text("").into();
        }
        container(self.feature.view().map(Message::Feature))
            .width(Fill)
            .height(Fill)
            .style(|theme| {
                ghost_shell_theme::surface(
                    theme,
                    self.config.launcher.background_opacity(),
                )
            })
            .into()
    }
}
