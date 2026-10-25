use std::collections::HashMap;

use ghost_shell_config::GreeterConfig;
use ghost_shell_wallpaper::WallpaperSource;
use iced::{Element, Subscription, Task, window::Id};
use iced_exwlshell::{
    reexport::{
        Anchor, KeyboardInteractivity, Layer, LayerSize, NewLayerShellSettings,
        OutputOption,
    },
    settings::{ExWlSettings, LayerShellSettings, StartMode},
    to_exwlshell_message,
};
use iced_wayland_subscriber::shell::{self, ShellEvent, ShellInfo, ShellType};

use crate::greeter::{Greeter, Message as Input};

#[to_exwlshell_message]
#[derive(Clone)]
enum Message {
    Shell(ShellEvent),
    Created(ShellInfo),
    Input(Input),
    Wallpaper(Result<WallpaperSource, String>),
    Theme(iced::theme::Mode),
    Tick,
}

impl std::fmt::Debug for Message {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("GreeterApplicationMessage")
    }
}

struct App {
    config: GreeterConfig,
    events: shell::ShellReceiver,
    windows: HashMap<u32, Id>,
    greeter: Greeter,
    wallpaper: WallpaperSource,
    clock: String,
    system_dark: bool,
}

pub fn run(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = ghost_shell_config::load_file::<GreeterConfig>(path)?;
    if config
        .session
        .command
        .first()
        .is_none_or(|command| command.trim().is_empty())
        || config
            .session
            .command
            .iter()
            .any(|argument| argument.contains('\0'))
    {
        return Err(
            "session.command must contain a nonempty command without NUL bytes".into(),
        );
    }
    if config
        .session
        .environment
        .iter()
        .any(|entry| {
            entry.contains('\0')
                || entry
                    .split_once('=')
                    .is_none_or(|(name, _)| name.is_empty())
        })
    {
        return Err(
            "session.environment must contain NAME=value entries without NUL bytes"
                .into(),
        );
    }
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
        "ghost-shell-greeter",
        App::update,
        App::view,
    )
    .subscription(App::subscription)
    .theme(|app: &App, _| ghost_shell_theme::theme(&app.config.theme, app.system_dark))
    .default_font(font)
    .default_text_size(font_size)
    .on_new_shell(|info| {
        (info.shell == ShellType::LayerShell).then_some(Message::Created(info))
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
    fn new(config: GreeterConfig, events: shell::ShellReceiver) -> (Self, Task<Message>) {
        let wallpaper =
            WallpaperSource::Solid(ghost_shell_theme::rgba(config.wallpaper.bg));
        let (greeter, connect) = Greeter::new(config.session.clone());
        let mut tasks = vec![
            connect.map(Message::Input),
            iced::system::theme().map(Message::Theme),
        ];
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
                windows: HashMap::new(),
                greeter,
                wallpaper,
                clock: jiff::Zoned::now()
                    .strftime("%H:%M")
                    .to_string(),
                system_dark: true,
            },
            Task::batch(tasks),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.events.listen().map(Message::Shell),
            iced::system::theme_changes().map(Message::Theme),
            iced::time::every(std::time::Duration::from_secs(60)).map(|_| Message::Tick),
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Shell(
                ShellEvent::OutputAdded(output) | ShellEvent::OutputUpdated(output),
            ) => {
                if let std::collections::hash_map::Entry::Vacant(entry) =
                    self.windows.entry(output.id)
                {
                    let id = Id::unique();
                    entry.insert(id);
                    return Task::done(Message::NewLayerShell {
                        id,
                        settings: NewLayerShellSettings {
                            layer: Layer::Overlay,
                            anchor: Anchor::all(),
                            size: LayerSize::FILL,
                            exclusive_zone: Some(-1),
                            keyboard_interactivity: KeyboardInteractivity::Exclusive,
                            output_option: OutputOption::GlobalName(output.id),
                            namespace: Some("ghost-shell-greeter".into()),
                            ..Default::default()
                        },
                    });
                }
            }
            Message::Shell(ShellEvent::OutputRemoved(output)) => {
                if let Some(id) = self.windows.remove(&output.id) {
                    return Task::done(Message::RemoveWindow(id));
                }
            }
            Message::Created(info) | Message::Shell(ShellEvent::NewShell(info))
                if info.shell == ShellType::LayerShell =>
            {
                return self.greeter.focus().map(Message::Input);
            }
            Message::Shell(ShellEvent::Closed(window)) => {
                self.windows.retain(|_, id| *id != window);
            }
            Message::Input(input) => {
                let task = self
                    .greeter
                    .update(input)
                    .map(Message::Input);
                if self.greeter.started() {
                    return iced::exit();
                }
                return task;
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
        self.greeter
            .view(&self.wallpaper, &self.clock)
            .map(Message::Input)
    }
}
