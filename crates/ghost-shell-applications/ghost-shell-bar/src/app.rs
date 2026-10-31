use std::collections::{BTreeMap, HashMap};

use futures::SinkExt as _;
use ghost_shell_bar::Bar;
use ghost_shell_config::{BarAppConfig, BarConfig};
use ghost_shell_dbus::MenuBackend as _;
use ghost_shell_wallpaper::WallpaperSource;
use ghost_shell_widget_audio::{AudioPopup, Message as AudioMessage};
use ghost_shell_widget_tray::{MenuState, TrayMessage};
use iced::{
    Element, Fill, Subscription, Task,
    widget::{container, operation, text},
    window::Id,
};
use iced_exwlshell::{
    actions::IcedNewPopupSettings,
    reexport::{
        Anchor, BlurOption, KeyboardInteractivity, Layer, LayerSize,
        NewLayerShellSettings, OutputOption, PixelSize, PopupAnchor,
        PopupConstraintAdjustment, PopupGravity, Position,
    },
    settings::{ExWlSettings, LayerShellSettings, StartMode},
    to_exwlshell_message,
};
use iced_wayland_subscriber::{
    ExWlOutputInfo,
    shell::{self, ShellEvent},
};

use crate::services;

#[to_exwlshell_message]
#[derive(Clone)]

enum Message {
    Shell(ShellEvent),
    Services(services::Event),
    Ipc(ghost_shell_ipc::subscription::Event),
    CommandResult(Result<(), String>),
    SystemResult(Id, Result<(), String>),
    Niri(
        Result<
            ghost_shell_niri::NiriEvent,
            std::sync::Arc<ghost_shell_niri::StreamError>,
        >,
    ),
    Bar(Id, ghost_shell_bar::Message),
    Wallpaper(u64, Result<WallpaperSource, String>),
    Configuration(Result<BarAppConfig, String>),
    TrayIcons(Result<Vec<ghost_shell_widget_tray::IconUpdate>, String>),
    Audio(ghost_shell_audio::AudioUpdate),
    Battery(Result<Option<&'static str>, String>),
    Tick,
    Key(Id, iced::keyboard::key::Named, bool),
    Cursor(Id, iced::Point),
    Closed(Id),
    FocusLost(Id),
    MenuLayout(Id, u64, Result<ghost_shell_dbus::MenuLayout, String>),
    MenuActivated(Result<(), String>),
    SystemTheme(iced::theme::Mode),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ShellMessage")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]

enum Surface {
    Wallpaper(u32),
    Bar(u32),
    Audio,
    Menu,
    Styling,
    SystemMenu,
}

enum PopupRequest {
    Audio(Id, ghost_shell_audio::AudioDirection),
    Menu(Id, ghost_shell_dbus::MenuId),
    Styling(Id),
    SystemMenu(Id),
}

struct Shell {
    config: BarAppConfig,
    file_config: BarAppConfig,
    configuration_error: Option<String>,
    styling: ghost_shell_widget_styling::Styling,
    wallpaper_generation: u64,
    events: shell::ShellReceiver,
    outputs: BTreeMap<u32, ExWlOutputInfo>,
    surfaces: HashMap<Id, Surface>,
    cursor: HashMap<Id, iced::Point>,
    popup_settings: HashMap<Id, IcedNewPopupSettings>,
    closing_popup: Option<Id>,
    pending_popup: Option<PopupRequest>,
    niri: ghost_shell_niri::NiriState,
    wallpaper: WallpaperSource,
    bar: Bar,
    bars_visible: bool,
    audio: Option<ghost_shell_audio::Audio>,
    audio_popup: AudioPopup,
    menu: Option<ghost_shell_dbus::Menu>,
    menu_state: Option<MenuState>,
    menu_generation: u64,
    menu_task: Option<iced::task::Handle>,
    loading_menu: Option<(Id, Id, iced::Point)>,
    error: Option<String>,
    system_dark: bool,
    system_busy: bool,
    system_error: Option<String>,
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config =
        ghost_shell_config::load::<BarAppConfig>("bar").unwrap_or_else(|error| {
            log::warn!("Configuration: {error}");

            BarAppConfig::default()
        });

    if config.bar.outputs.is_empty() {
        log::info!("No output filter configured; showing the bar on each output");
    }

    let font = ghost_shell_theme::font(&config.general.font_family);

    let font_size = config.general.font_size;

    let (shell_broadcast, events) = shell::channel();

    iced_exwlshell::daemon(
        move || Shell::new(config.clone(), events.clone()),
        "ghost-shell-bar",
        Shell::update,
        Shell::view,
    )
    .subscription(Shell::subscription)
    .theme(|shell: &Shell, window| {
        let mut config = shell.config.theme.clone();
        // Override only the panel accent; palette edits and exported colors stay intact.
        if shell.surfaces.get(&window) == Some(&Surface::Styling) {
            config.dark.base0d = config.dark.base08;
            config.light.base0d = config.light.base08;
        }
        ghost_shell_theme::theme(&config, shell.system_dark)
    })
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
    .redraw_scope(|message| match message {
        Message::Cursor(..) => iced_exwlshell::redraw::Scope::None,
        Message::Bar(
            _,
            ghost_shell_bar::Message::Styling(_) | ghost_shell_bar::Message::Theme(_),
        ) => iced_exwlshell::redraw::Scope::All,
        Message::Bar(window, _) => iced_exwlshell::redraw::Scope::Window(*window),
        _ => iced_exwlshell::redraw::Scope::All,
    })
    .run()?;

    Ok(())
}

impl Shell {
    fn new(config: BarAppConfig, events: shell::ShellReceiver) -> (Self, Task<Message>) {
        let wallpaper =
            WallpaperSource::Solid(ghost_shell_theme::rgba(config.wallpaper.bg));

        let mut tasks = vec![
            iced::system::theme().map(Message::SystemTheme),
            Task::perform(
                async {
                    tokio::task::spawn_blocking(ghost_shell_widget_power::battery)
                        .await
                        .unwrap_or_else(|error| Err(error.to_string()))
                },
                Message::Battery,
            ),
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
                |result| Message::Wallpaper(0, result),
            ));
        }

        let audio = match ghost_shell_audio::Audio::start() {
            Ok((audio, mut receiver)) => {
                tasks.push(Task::run(
                    iced::stream::channel::<ghost_shell_audio::AudioUpdate>(
                        128,
                        async move |mut output| {
                            while let Some(event) = receiver.recv().await {
                                if output.send(event).await.is_err() {
                                    return;
                                }
                            }

                            if output
                                .send(ghost_shell_audio::AudioUpdate::Connection(
                                    ghost_shell_audio::AudioConnectionStatus::Stopped,
                                ))
                                .await
                                .is_err()
                            {
                                log::debug!("Audio receiver closed");
                            }
                        },
                    ),
                    Message::Audio,
                ));

                Some(audio)
            }
            Err(error) => {
                log::warn!("Audio: {error:#}");

                None
            }
        };

        let bar = Bar {
            tray: ghost_shell_widget_tray::Tray::new(
                config.theme.icons.clone(),
                config.bar.widgets.icon_size,
            ),
            ..Bar::default()
        };
        let mut shell = Self {
            file_config: config.clone(),
            styling: ghost_shell_widget_styling::Styling::new(&config),
            configuration_error: None,
            wallpaper_generation: 0,
            config,
            events,
            outputs: BTreeMap::new(),
            surfaces: HashMap::new(),
            cursor: HashMap::new(),
            popup_settings: HashMap::new(),
            closing_popup: None,
            pending_popup: None,
            niri: Default::default(),
            wallpaper,
            bar,
            bars_visible: true,
            audio,
            audio_popup: Default::default(),
            menu: None,
            menu_state: None,
            menu_generation: 0,
            menu_task: None,
            loading_menu: None,
            error: None,
            system_dark: true,
            system_busy: false,
            system_error: None,
        };

        shell.clock();

        (shell, Task::batch(tasks))
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.events.listen().map(Message::Shell),
            Subscription::run(configuration_changes).map(Message::Configuration),
            Subscription::run(services::events).map(Message::Services),
            ghost_shell_ipc::subscription::listen(ghost_shell_ipc::Endpoint::Bar)
                .map(Message::Ipc),
            Subscription::run(ghost_shell_niri::events).map(Message::Niri),
            ghost_shell_widget_clock::subscription(&self.config.clock.format)
                .map(|()| Message::Tick),
            iced::system::theme_changes().map(Message::SystemTheme),
            iced::event::listen_with(|event, status, window| match event {
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
                    Some(Message::Key(window, key, modifiers.shift()))
                }
                iced::Event::Window(iced::window::Event::Unfocused) => {
                    Some(Message::FocusLost(window))
                }
                iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                    Some(Message::Cursor(window, position))
                }
                _ => None,
            }),
        ])
    }

    fn apply_config(&mut self, config: BarAppConfig) -> Task<Message> {
        if self.config == config {
            return Task::none();
        }
        let previous = std::mem::replace(&mut self.config, config);
        let mut tasks = Vec::new();
        let icons = self.bar.tray.configure(
            self.config.theme.icons.clone(),
            self.config.bar.widgets.icon_size,
        );
        if !icons.is_empty() {
            tasks.push(Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        ghost_shell_widget_tray::resolve_icons(icons)
                    })
                    .await
                    .map_err(|error| error.to_string())
                },
                Message::TrayIcons,
            ));
        }
        if self.config.clock != previous.clock {
            self.clock();
        }
        if self.config.wallpaper != previous.wallpaper {
            self.wallpaper_generation = self.wallpaper_generation.wrapping_add(1);
            let generation = self.wallpaper_generation;
            if let Some(path) = self.config.wallpaper.path.clone() {
                tasks.push(Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            WallpaperSource::load(std::path::Path::new(&path))
                                .map_err(|error| error.to_string())
                        })
                        .await
                        .unwrap_or_else(|error| Err(error.to_string()))
                    },
                    move |result| Message::Wallpaper(generation, result),
                ));
            } else {
                self.wallpaper = WallpaperSource::Solid(ghost_shell_theme::rgba(
                    self.config.wallpaper.bg,
                ));
            }
        }
        let bars: Vec<_> = self
            .surfaces
            .iter()
            .filter_map(|(window, surface)| match surface {
                Surface::Bar(output) => Some((*window, *output)),
                _ => None,
            })
            .collect();
        for (window, output) in bars {
            let enabled = self
                .outputs
                .get(&output)
                .and_then(|output| output.name.as_deref())
                .is_some_and(|name| self.bar_config(name).is_some());
            if !enabled {
                self.pending_popup = None;
                self.cancel_loading_menu();
                let children: Vec<_> = self
                    .popup_settings
                    .iter()
                    .filter_map(|(id, settings)| {
                        (settings.parent == Some(window)).then_some(*id)
                    })
                    .collect();
                for child in children {
                    tasks.push(self.close(child));
                }
                tasks.push(self.close(window));
                continue;
            }
            if self.config.bar.height != previous.bar.height {
                tasks.push(Task::done(Message::LayoutChange {
                    id: window,
                    anchor: Anchor::Top | Anchor::Left | Anchor::Right,
                    size: LayerSize::fill_width(self.config.bar.height.max(1.0) as u32),
                }));
            }
            if self.config.bar.exclusive_zone != previous.bar.exclusive_zone {
                tasks.push(Task::done(Message::ExclusiveZoneChange {
                    id: window,
                    zone_size: self.config.bar.exclusive_zone as i32,
                }));
            }
            if !self.config.bar.decoration.blur && previous.bar.decoration.blur {
                tasks.push(Task::done(Message::BlurOptionChange {
                    id: window,
                    option: BlurOption::None,
                }));
            }
        }
        if self.config.bar.outputs != previous.bar.outputs {
            tasks.push(self.reconcile());
        } else if self.config.bar.height != previous.bar.height {
            tasks.push(self.update_styling_position());
        }
        Task::batch(tasks)
    }

    fn clock(&mut self) {
        let now = jiff::Zoned::now();

        self.bar
            .clock
            .update(&now, &self.config.clock.format);
    }

    fn find(&self, surface: Surface) -> Option<Id> {
        self.surfaces
            .iter()
            .find_map(|(id, candidate)| (*candidate == surface).then_some(*id))
    }

    fn error(&mut self, error: String) {
        log::error!("{error}");

        self.error = Some(error);
    }

    fn create(
        &mut self,
        surface: Surface,
        settings: NewLayerShellSettings,
    ) -> Task<Message> {
        let id = Id::unique();

        self.surfaces.insert(id, surface);

        Task::done(Message::NewLayerShell { id, settings })
    }

    fn bar_config(&self, output: &str) -> Option<&BarConfig> {
        (self.config.bar.outputs.is_empty()
            || self.config.bar.outputs.contains_key(output))
        .then_some(&self.config.bar)
    }

    fn reconcile(&mut self) -> Task<Message> {
        let mut tasks = Vec::new();

        let outputs: Vec<_> = self
            .outputs
            .iter()
            .map(|(id, info)| (*id, info.name.clone()))
            .collect();

        for (output, name) in outputs {
            if self
                .find(Surface::Wallpaper(output))
                .is_none()
            {
                tasks.push(self.create(
                    Surface::Wallpaper(output),
                    NewLayerShellSettings {
                        layer: Layer::Background,
                        anchor: Anchor::all(),
                        size: LayerSize::FILL,
                        exclusive_zone: Some(-1),
                        keyboard_interactivity: KeyboardInteractivity::None,
                        events_transparent: true,
                        output_option: OutputOption::GlobalName(output),
                        namespace: Some("ghost-shell-wallpaper".into()),
                        ..Default::default()
                    },
                ));
            }

            if self.bars_visible
                && let Some(config) = name
                    .as_ref()
                    .and_then(|name| self.bar_config(name))
                    .cloned()
            {
                if self.find(Surface::Bar(output)).is_none() {
                    tasks.push(self.create(
                        Surface::Bar(output),
                        NewLayerShellSettings {
                            layer: Layer::Bottom,
                            anchor: Anchor::Top | Anchor::Left | Anchor::Right,
                            size: LayerSize::fill_width(config.height.max(1.0) as u32),
                            exclusive_zone: Some(config.exclusive_zone as i32),
                            // Niri only grants popup keyboard grabs to focusable parent layers.
                            keyboard_interactivity: KeyboardInteractivity::OnDemand,
                            output_option: OutputOption::GlobalName(output),
                            namespace: Some("ghost-shell-bar".into()),
                            blur_option: BlurOption::None,
                            ..Default::default()
                        },
                    ));
                }
            }
        }

        tasks.push(self.update_styling_position());
        Task::batch(tasks)
    }

    fn close(&mut self, window: Id) -> Task<Message> {
        if matches!(
            self.surfaces.get(&window),
            Some(Surface::Audio | Surface::Menu | Surface::Styling | Surface::SystemMenu)
        ) {
            if self.closing_popup == Some(window) {
                return Task::none();
            }
            // A close request only schedules destruction in Waycrate. Keep the popup
            // tracked until Closed so a replacement cannot grab above a live sibling.
            self.closing_popup = Some(window);
        } else {
            self.closed(window);
        }

        iced::window::close(window)
    }

    fn cancel_loading_menu(&mut self) {
        if self.loading_menu.take().is_some() {
            self.menu_generation = self.menu_generation.wrapping_add(1);
            if let Some(task) = self.menu_task.take() {
                task.abort();
            }
        }
    }

    fn closed(&mut self, window: Id) {
        if self
            .loading_menu
            .is_some_and(|(_, parent, _)| parent == window)
        {
            self.cancel_loading_menu();
        }
        if self.closing_popup == Some(window) {
            self.closing_popup = None;
        }
        if self.surfaces.remove(&window) == Some(Surface::Menu) {
            self.menu_state = None;

            self.menu_generation = self.menu_generation.wrapping_add(1);

            if let Some(task) = self.menu_task.take() {
                task.abort();
            }
        }

        self.cursor.remove(&window);

        self.popup_settings.remove(&window);
    }

    fn popup(
        &mut self,
        parent: Id,
        surface: Surface,
        width: u32,
        height: u32,
    ) -> (Id, Task<Message>) {
        let position = self
            .cursor
            .get(&parent)
            .copied()
            .unwrap_or_default();

        let id = Id::unique();

        self.surfaces.insert(id, surface);

        let settings = if surface == Surface::Styling {
            self.styling_settings(parent)
        } else {
            IcedNewPopupSettings::new(
                parent,
                PixelSize::px(width, height),
                Position::new(position.x as i32, position.y as i32),
                PixelSize::px(1, 1),
            )
            .anchor(PopupAnchor::BottomRight)
            .gravity(PopupGravity::BottomLeft)
            .constraint_adjustment(
                PopupConstraintAdjustment::SlideX | PopupConstraintAdjustment::SlideY,
            )
        };

        self.popup_settings.insert(id, settings);

        (id, Task::done(Message::NewPopUp { id, settings }))
    }

    fn styling_settings(&self, parent: Id) -> IcedNewPopupSettings {
        let logical_size = self
            .surfaces
            .get(&parent)
            .and_then(|surface| match surface {
                Surface::Bar(output) => self
                    .outputs
                    .get(output)
                    .and_then(|output| output.logical_size),
                _ => None,
            });
        let (width, height) = logical_size
            .map(|(width, height)| {
                (
                    (width - 16).clamp(1, 480) as u32,
                    (height - self.config.bar.height.ceil() as i32 - 16).clamp(1, 640)
                        as u32,
                )
            })
            .unwrap_or((480, 640));
        let position = self
            .cursor
            .get(&parent)
            .copied()
            .unwrap_or_default();
        IcedNewPopupSettings::new(
            parent,
            PixelSize::px(width, height),
            Position::new(position.x as i32, self.config.bar.height.ceil() as i32),
            PixelSize::px(1, 1),
        )
        .anchor(PopupAnchor::BottomRight)
        .gravity(PopupGravity::BottomLeft)
        .constraint_adjustment(
            PopupConstraintAdjustment::SlideX | PopupConstraintAdjustment::SlideY,
        )
    }

    fn update_styling_position(&mut self) -> Task<Message> {
        let Some(window) = self.find(Surface::Styling) else {
            return Task::none();
        };
        let Some(parent) = self
            .popup_settings
            .get(&window)
            .and_then(|settings| settings.parent)
        else {
            return Task::none();
        };
        if !self.surfaces.contains_key(&parent) {
            return Task::none();
        }
        let settings = self.styling_settings(parent);
        if self.popup_settings.get(&window) == Some(&settings) {
            return Task::none();
        }
        self.popup_settings.insert(window, settings);
        Task::done(Message::PopUpReposition {
            id: window,
            settings,
        })
    }

    fn request_popup(&mut self, request: PopupRequest) -> Task<Message> {
        self.cancel_loading_menu();
        let parent = match &request {
            PopupRequest::Audio(parent, _)
            | PopupRequest::Menu(parent, _)
            | PopupRequest::Styling(parent)
            | PopupRequest::SystemMenu(parent) => *parent,
        };
        if !self.bars_visible
            || !matches!(self.surfaces.get(&parent), Some(Surface::Bar(_)))
        {
            return Task::none();
        }
        if let Some(window) = self
            .find(Surface::Menu)
            .or_else(|| self.find(Surface::Audio))
            .or_else(|| self.find(Surface::Styling))
            .or_else(|| self.find(Surface::SystemMenu))
        {
            self.pending_popup = Some(request);
            return self.close(window);
        }
        match request {
            PopupRequest::Audio(parent, direction) => {
                let (width, height) = self.audio_popup.open(
                    direction,
                    self.audio
                        .as_ref()
                        .map(|audio| audio.state()),
                );
                self.popup(parent, Surface::Audio, width, height)
                    .1
            }
            PopupRequest::Menu(parent, menu) => self.open_menu(parent, menu),
            PopupRequest::SystemMenu(parent) => {
                let (width, height) =
                    ghost_shell_widget_menu::initial_size(self.config.general.font_size);
                self.popup(parent, Surface::SystemMenu, width, height)
                    .1
            }
            PopupRequest::Styling(parent) => {
                self.popup(parent, Surface::Styling, 480, 640)
                    .1
            }
        }
    }

    fn open_menu(
        &mut self,
        parent: Id,
        menu_id: ghost_shell_dbus::MenuId,
    ) -> Task<Message> {
        let Some(menu) = self.menu.clone() else {
            return Task::none();
        };

        let window = Id::unique();
        let position = self
            .cursor
            .get(&parent)
            .copied()
            .unwrap_or_default();
        // Wait for the layout before creating a Wayland surface, avoiding a
        // visible placeholder and compositor placement based on the wrong size.
        self.loading_menu = Some((window, parent, position));

        self.menu_generation = self.menu_generation.wrapping_add(1);

        let generation = self.menu_generation;

        let stream = iced::stream::channel::<Result<ghost_shell_dbus::MenuLayout, String>>(
            8,
            async move |mut output| {
                loop {
                    match menu.open(menu_id.clone()).await {
                        Ok(mut session) => {
                            if output
                                .send(Ok(session.layout.clone()))
                                .await
                                .is_err()
                            {
                                return;
                            }

                            session.invalidated().await;
                        }
                        Err(error) => {
                            if output
                                .send(Err(error.to_string()))
                                .await
                                .is_err()
                            {
                                log::debug!("Menu receiver closed");
                            }

                            return;
                        }
                    }
                }
            },
        );

        let (watch, handle) = Task::run(stream, move |layout| {
            Message::MenuLayout(window, generation, layout)
        })
        .abortable();

        self.menu_task = Some(handle);

        watch
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CommandResult(Err(error)) => self.error(error),
            Message::CommandResult(Ok(())) => {}
            Message::Ipc(ghost_shell_ipc::subscription::Event::Failed(error)) => {
                self.error(error);
                return iced::exit();
            }
            Message::Ipc(ghost_shell_ipc::subscription::Event::Request(incoming)) => {
                if let Some(request) = incoming.take() {
                    use ghost_shell_ipc::{BarAction, Request, Response};
                    let task = match &request.request {
                        Request::Ping => Task::none(),
                        Request::Shutdown => {
                            return ghost_shell_ipc::subscription::shutdown(request);
                        }
                        Request::Bar { action } => {
                            self.bars_visible = match action {
                                BarAction::Open => true,
                                BarAction::Close => false,
                                BarAction::Toggle => !self.bars_visible,
                            };
                            if self.bars_visible {
                                self.reconcile()
                            } else {
                                let windows: Vec<_> = self
                                    .surfaces
                                    .iter()
                                    .filter_map(|(id, surface)| {
                                        (!matches!(surface, Surface::Wallpaper(_)))
                                            .then_some(*id)
                                    })
                                    .collect();
                                Task::batch(
                                    windows
                                        .into_iter()
                                        .map(|window| self.close(window))
                                        .collect::<Vec<_>>(),
                                )
                            }
                        }
                        _ => {
                            request.respond(Err(
                                "Command is not supported by ghost-shell-bar".into(),
                            ));
                            return Task::none();
                        }
                    };
                    request.respond(Ok(Response::Handled));
                    return task;
                }
            }
            Message::Shell(
                ShellEvent::OutputAdded(output) | ShellEvent::OutputUpdated(output),
            ) => {
                self.outputs.insert(output.id, output);

                return self.reconcile();
            }
            Message::Shell(ShellEvent::OutputRemoved(output)) => {
                self.outputs.remove(&output.id);

                let affected: Vec<_> = self.surfaces.iter().filter_map(|(window, surface)| matches!(surface, Surface::Wallpaper(id) | Surface::Bar(id) if *id == output.id).then_some(*window)).collect();

                return Task::batch(
                    affected
                        .into_iter()
                        .map(|window| self.close(window))
                        .collect::<Vec<_>>(),
                );
            }
            Message::Shell(ShellEvent::Closed(window)) | Message::Closed(window) => {
                self.closed(window);
                if self.find(Surface::Menu).is_none()
                    && self.find(Surface::Audio).is_none()
                    && self.find(Surface::Styling).is_none()
                    && self.find(Surface::SystemMenu).is_none()
                {
                    if let Some(request) = self.pending_popup.take() {
                        return self.request_popup(request);
                    }
                }
            }
            Message::Shell(_) => {}
            Message::Wallpaper(generation, result)
                if generation == self.wallpaper_generation =>
            {
                match result {
                    Ok(source) => self.wallpaper = source,
                    Err(error) => self.error(format!("Wallpaper: {error}")),
                }
            }
            Message::Wallpaper(..) => {}
            Message::Configuration(Ok(config)) => {
                self.configuration_error = None;
                if config != self.file_config {
                    self.file_config = config.clone();
                    self.styling = ghost_shell_widget_styling::Styling::new(&config);
                    return self.apply_config(config);
                }
            }
            Message::Configuration(Err(error)) => {
                log::warn!("Configuration reload: {error}");
                self.configuration_error = Some(format!(
                    "Configuration reload failed; keeping current settings: {error}"
                ));
            }
            Message::Niri(update) => match update {
                Ok(event) => self.niri.update(event),
                Err(error) => {
                    self.niri = Default::default();
                    self.error(format!("Niri: {error}"));
                }
            },
            Message::TrayIcons(result) => {
                let errors = match result {
                    Ok(updates) => self.bar.tray.apply_icons(updates),
                    Err(error) => vec![error],
                };
                if !errors.is_empty() {
                    let error = format!("Tray icon refresh: {}", errors.join("; "));
                    log::warn!("{error}");
                    self.styling.error = Some(error);
                }
            }
            Message::Services(event) => match event {
                services::Event::Item(item) => self.bar.tray.update(item),
                services::Event::Removed(id) => self.bar.tray.remove(&id),
                services::Event::Menu(menu) => self.menu = Some(menu),
                services::Event::Error(error) => self.error(error),
            },
            Message::Audio(update) => {
                if let Some(audio) = &mut self.audio {
                    for event in audio.apply(update) {
                        self.audio_popup.handle_event(event);
                    }
                }
            }
            Message::Bar(window, message) => return self.bar_message(window, message),
            Message::Battery(Ok(battery)) => self.bar.power.update(battery),
            Message::Battery(Err(error)) => log::warn!("Battery: {error}"),
            Message::Tick => self.clock(),
            Message::Cursor(window, position) => {
                self.cursor.insert(window, position);
            }
            Message::Key(window, key, shift) => return self.key(window, key, shift),
            Message::FocusLost(window) => {
                if matches!(
                    self.surfaces.get(&window),
                    Some(
                        Surface::Audio
                            | Surface::Menu
                            | Surface::Styling
                            | Surface::SystemMenu
                    )
                ) {
                    return self.close(window);
                }
            }
            Message::MenuLayout(window, generation, result)
                if generation == self.menu_generation
                    && (self.surfaces.get(&window) == Some(&Surface::Menu)
                        || self
                            .loading_menu
                            .is_some_and(|(id, _, _)| id == window)) =>
            {
                match result {
                    Ok(layout) => {
                        if let Some((_, parent, position)) = self.loading_menu.take() {
                            if !self.bars_visible
                                || !matches!(
                                    self.surfaces.get(&parent),
                                    Some(Surface::Bar(_))
                                )
                            {
                                if let Some(task) = self.menu_task.take() {
                                    task.abort();
                                }
                                return Task::none();
                            }
                            let state = MenuState::new(layout);
                            let (width, height) = state.size();
                            let settings = IcedNewPopupSettings::new(
                                parent,
                                PixelSize::px(width, height),
                                Position::new(position.x as i32, position.y as i32),
                                PixelSize::px(1, 1),
                            )
                            .anchor(PopupAnchor::BottomRight)
                            .gravity(PopupGravity::BottomLeft)
                            .constraint_adjustment(
                                PopupConstraintAdjustment::SlideX
                                    | PopupConstraintAdjustment::SlideY,
                            );
                            self.menu_state = Some(state);
                            self.surfaces.insert(window, Surface::Menu);
                            self.popup_settings.insert(window, settings);
                            return Task::done(Message::NewPopUp {
                                id: window,
                                settings,
                            });
                        }
                        if let Some(state) = &mut self.menu_state {
                            state.replace(layout);
                        } else {
                            self.menu_state = Some(MenuState::new(layout));
                        }

                        return self.resize_menu(window);
                    }
                    Err(error) => {
                        self.cancel_loading_menu();
                        self.error(format!("Menu: {error}"));
                    }
                }
            }
            Message::MenuLayout(..) => {}
            Message::MenuActivated(result) => {
                if let Err(error) = result {
                    self.error(error);
                }
            }
            Message::SystemResult(window, result) => {
                self.system_busy = false;
                match result {
                    Ok(())
                        if self.surfaces.get(&window) == Some(&Surface::SystemMenu) =>
                    {
                        return self.close(window);
                    }
                    Ok(()) => {}
                    Err(error) => {
                        log::error!("System action: {error}");
                        self.system_error = Some(error);
                    }
                }
            }
            Message::SystemTheme(mode) => {
                self.system_dark = mode != iced::theme::Mode::Light
            }
            _ => {}
        }

        Task::none()
    }

    fn resize_menu(&mut self, window: Id) -> Task<Message> {
        let Some(state) = &self.menu_state else {
            return Task::none();
        };
        self.resize_popup(window, state.size())
    }

    fn resize_popup(&mut self, window: Id, (width, height): (u32, u32)) -> Task<Message> {
        let Some(settings) = self.popup_settings.get_mut(&window) else {
            return Task::none();
        };
        let size = PixelSize::px(width, height);
        if settings.size == size {
            return Task::none();
        }
        settings.size = size;
        Task::done(Message::PopUpReposition {
            id: window,
            settings: *settings,
        })
    }

    fn bar_message(
        &mut self,
        window: Id,
        message: ghost_shell_bar::Message,
    ) -> Task<Message> {
        use ghost_shell_bar::Message as BarMessage;

        match message {
            BarMessage::Theme(ghost_shell_widget_theme::Message::Toggle) => {
                use ghost_shell_config::ThemeMode;
                let dark = match self.config.theme.mode {
                    ThemeMode::Dark => true,
                    ThemeMode::Light => false,
                    ThemeMode::System => self.system_dark,
                };
                let mut config = self.config.clone();
                self.styling.update(
                    ghost_shell_widget_styling::Message::Mode(if dark {
                        ThemeMode::Light
                    } else {
                        ThemeMode::Dark
                    }),
                    &mut config,
                );
                return self.apply_config(config);
            }
            BarMessage::Styling(message) => {
                use ghost_shell_widget_styling::Message as StylingMessage;
                match message {
                    StylingMessage::Close => return self.close(window),
                    StylingMessage::Reset => {
                        self.styling =
                            ghost_shell_widget_styling::Styling::new(&self.file_config);
                        return self.apply_config(self.file_config.clone());
                    }
                    StylingMessage::Copy => {
                        match ghost_shell_widget_styling::export_nix(&self.config) {
                            Ok(nix) => {
                                self.styling.notice = Some("Copied Nix settings. Merge these sections into your configuration.".into());
                                return iced::clipboard::write(nix);
                            }
                            Err(error) => self.styling.error = Some(error.to_string()),
                        }
                    }
                    message => {
                        let mut config = self.config.clone();
                        if self.styling.update(message, &mut config) {
                            return self.apply_config(config);
                        }
                    }
                }
            }
            BarMessage::Decorations(decorations) => {
                if matches!(self.surfaces.get(&window), Some(Surface::Bar(_)))
                    && self.config.bar.decoration.blur
                {
                    let regions = crate::blur::regions(&decorations);
                    return Task::done(Message::BlurOptionChange {
                        id: window,
                        option: if regions.is_empty() {
                            BlurOption::None
                        } else {
                            BlurOption::Region(regions)
                        },
                    });
                }
            }
            BarMessage::Menu(message) => {
                use ghost_shell_widget_menu::Message as MenuMessage;
                if let MenuMessage::Resize(width, height) = message {
                    if self.surfaces.get(&window) == Some(&Surface::SystemMenu) {
                        return self.resize_popup(window, (width, height));
                    }
                    return Task::none();
                }
                if matches!(message, MenuMessage::Open) {
                    if let Some(popup) = self.find(Surface::SystemMenu) {
                        self.pending_popup = None;
                        return self.close(popup);
                    }
                    return self.request_popup(PopupRequest::SystemMenu(window));
                }
                if self.surfaces.get(&window) != Some(&Surface::SystemMenu)
                    || self.system_busy
                {
                    return Task::none();
                }
                if matches!(message, MenuMessage::Styling) {
                    if let Some(parent) = self
                        .popup_settings
                        .get(&window)
                        .and_then(|settings| settings.parent)
                    {
                        return self.request_popup(PopupRequest::Styling(parent));
                    }
                    return Task::none();
                }
                self.system_busy = true;
                self.system_error = None;
                return Task::perform(system_action(message), move |result| {
                    Message::SystemResult(window, result)
                });
            }
            BarMessage::Audio(AudioMessage::Open(direction)) => {
                if let Some(window) = self.find(Surface::Audio)
                    && self.audio_popup.direction() == direction
                {
                    self.pending_popup = None;
                    return self.close(window);
                }

                return self.request_popup(PopupRequest::Audio(window, direction));
            }
            BarMessage::Tray(TrayMessage::Open(menu)) => {
                return self.request_popup(PopupRequest::Menu(window, menu));
            }
            BarMessage::Tray(TrayMessage::Dismiss)
            | BarMessage::Audio(AudioMessage::Dismiss) => {
                return self.close(window);
            }
            BarMessage::Tray(message) => {
                if let Some(state) = &mut self.menu_state {
                    if let Some(item) = state.update(&message) {
                        let id = state.layout.id.clone();

                        if let Some(menu) = self.menu.clone() {
                            return Task::batch([
                                self.close(window),
                                Task::perform(
                                    async move {
                                        menu.activate(id, item)
                                            .await
                                            .map_err(|error| error.to_string())
                                    },
                                    Message::MenuActivated,
                                ),
                            ]);
                        }
                    }
                }
                return self.resize_menu(window);
            }
            BarMessage::Audio(AudioMessage::Resize(width, height)) => {
                if self.surfaces.get(&window) == Some(&Surface::Audio) {
                    return self.resize_popup(window, (width, height));
                }
            }
            BarMessage::Audio(message) => self
                .audio_popup
                .update(message, self.audio.as_ref()),
        }

        Task::none()
    }

    fn key(
        &mut self,
        window: Id,
        key: iced::keyboard::key::Named,
        shift: bool,
    ) -> Task<Message> {
        use iced::keyboard::key::Named;

        let surface = self.surfaces.get(&window).copied();

        if key == Named::Escape
            && matches!(
                surface,
                Some(
                    Surface::Audio
                        | Surface::Menu
                        | Surface::Styling
                        | Surface::SystemMenu
                )
            )
        {
            return self.close(window);
        }

        if key == Named::Tab {
            return if shift {
                operation::focus_previous()
            } else {
                operation::focus_next()
            };
        }

        Task::none()
    }

    fn view(&self, window: Id) -> Element<'_, Message> {
        match self.surfaces.get(&window) {
            Some(Surface::Wallpaper(_)) => self.wallpaper.view(),
            Some(Surface::Bar(output)) => match self
                .outputs
                .get(output)
                .and_then(|output| output.name.as_ref())
                .and_then(|name| {
                    self.bar_config(name)
                        .map(|config| (name, config))
                }) {
                Some((name, config)) => self
                    .bar
                    .view(
                        config,
                        &self.config.general,
                        name,
                        &self.niri,
                        self.audio
                            .as_ref()
                            .map(|audio| audio.state()),
                    )
                    .map(move |message| Message::Bar(window, message)),
                None => text("").into(),
            },
            Some(Surface::SystemMenu) => ghost_shell_widget_menu::popup(
                self.system_error.as_deref(),
                self.system_busy,
                self.configuration_error.is_some(),
                ghost_shell_theme::font(&self.config.general.font_family),
                self.config.general.font_size,
            )
            .map(move |message| {
                Message::Bar(window, ghost_shell_bar::Message::Menu(message))
            }),
            Some(Surface::Styling) => self
                .styling
                .view(
                    &self.config,
                    self.configuration_error
                        .as_deref()
                        .or(self.error.as_deref()),
                )
                .map(move |message| {
                    Message::Bar(window, ghost_shell_bar::Message::Styling(message))
                }),
            Some(Surface::Audio) => self
                .audio_popup
                .view(
                    self.audio
                        .as_ref()
                        .map(|audio| audio.state()),
                    ghost_shell_theme::font(&self.config.general.font_family),
                    self.config.general.font_size,
                )
                .map(move |message| {
                    Message::Bar(window, ghost_shell_bar::Message::Audio(message))
                }),
            Some(Surface::Menu) => match &self.menu_state {
                Some(state) => state
                    .view(
                        ghost_shell_theme::font(&self.config.general.font_family),
                        self.config.general.font_size,
                    )
                    .map(move |message| {
                        Message::Bar(window, ghost_shell_bar::Message::Tray(message))
                    }),
                None => text(self.error.as_deref().unwrap_or("Loading…")).into(),
            },
            None => container(text(""))
                .width(Fill)
                .height(Fill)
                .style(|_| container::Style::default().background(iced::Color::BLACK))
                .into(),
        }
    }
}

async fn system_action(action: ghost_shell_widget_menu::Message) -> Result<(), String> {
    use ghost_shell_widget_menu::Message;
    let command = match action {
        Message::Lock => {
            return ghost_shell_ipc::call(
                ghost_shell_ipc::Endpoint::Lockscreen,
                ghost_shell_ipc::Request::Lock {
                    action: ghost_shell_ipc::LockAction::Lock,
                },
            )
            .await
            .map_err(|error| format!("Could not contact lockscreen: {error}"))?
            .map(|_| ());
        }
        Message::Shutdown => "poweroff",
        Message::Reboot => "reboot",
        _ => return Err("Unsupported system action".into()),
    };
    let output = tokio::process::Command::new("systemctl")
        .args(["--no-ask-password", command])
        .output()
        .await
        .map_err(|error| format!("Could not run systemctl {command}: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "systemctl {command} failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn configuration_changes() -> impl futures::Stream<Item = Result<BarAppConfig, String>> {
    ghost_shell_config::watch::changes("bar")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> Shell {
        let config = BarAppConfig::default();
        let (_, events) = shell::channel();
        Shell {
            file_config: config.clone(),
            styling: ghost_shell_widget_styling::Styling::new(&config),
            configuration_error: None,
            wallpaper_generation: 0,
            config,
            events,
            outputs: BTreeMap::new(),
            surfaces: HashMap::new(),
            cursor: HashMap::new(),
            popup_settings: HashMap::new(),
            closing_popup: None,
            pending_popup: None,
            niri: Default::default(),
            wallpaper: WallpaperSource::Solid(iced::Color::TRANSPARENT),
            bar: Bar::default(),
            bars_visible: true,
            audio: None,
            audio_popup: Default::default(),
            menu: None,
            menu_state: None,
            menu_generation: 0,
            menu_task: None,
            loading_menu: None,
            error: None,
            system_dark: true,
            system_busy: false,
            system_error: None,
        }
    }

    #[test]
    fn preview_reset_and_invalid_reload_preserve_the_correct_baseline() {
        let mut shell = shell();
        let original = shell.file_config.clone();
        let mut preview = original.clone();
        preview.bar.decoration.opacity = 0.3;
        drop(shell.apply_config(preview.clone()));
        assert_eq!(shell.file_config, original);
        drop(shell.update(Message::Configuration(Err("invalid file".into()))));
        assert_eq!(shell.config, preview);
        assert!(shell.configuration_error.is_some());
        drop(shell.bar_message(
            Id::unique(),
            ghost_shell_bar::Message::Styling(ghost_shell_widget_styling::Message::Reset),
        ));
        assert_eq!(shell.config, original);

        let mut replacement = original;
        replacement.bar.height = 40.0;
        drop(shell.update(Message::Configuration(Ok(replacement.clone()))));
        assert_eq!(shell.file_config, replacement);
        assert_eq!(shell.config, replacement);
        assert!(shell.configuration_error.is_none());
    }

    #[test]
    fn system_menu_hands_styling_back_to_its_bar_after_closing() {
        let mut shell = shell();
        let parent = Id::unique();
        shell
            .surfaces
            .insert(parent, Surface::Bar(1));
        drop(shell.request_popup(PopupRequest::SystemMenu(parent)));
        let menu = shell.find(Surface::SystemMenu).unwrap();
        drop(shell.bar_message(
            menu,
            ghost_shell_bar::Message::Menu(ghost_shell_widget_menu::Message::Styling),
        ));
        assert_eq!(shell.closing_popup, Some(menu));
        assert!(shell.find(Surface::Styling).is_none());
        drop(shell.update(Message::Closed(menu)));
        let styling = shell.find(Surface::Styling).unwrap();
        assert_eq!(
            shell
                .popup_settings
                .get(&styling)
                .and_then(|settings| settings.parent),
            Some(parent)
        );
        assert!(shell.find(Surface::SystemMenu).is_none());
    }

    #[test]
    fn failed_system_action_keeps_menu_open_and_allows_retry() {
        let mut shell = shell();
        let parent = Id::unique();
        shell
            .surfaces
            .insert(parent, Surface::Bar(1));
        drop(shell.request_popup(PopupRequest::SystemMenu(parent)));
        let menu = shell.find(Surface::SystemMenu).unwrap();
        shell.system_busy = true;
        drop(shell.update(Message::SystemResult(menu, Err("Access denied".into()))));
        assert!(!shell.system_busy);
        assert_eq!(shell.system_error.as_deref(), Some("Access denied"));
        assert_eq!(shell.find(Surface::SystemMenu), Some(menu));
        assert!(shell.closing_popup.is_none());
    }

    #[test]
    fn theme_toggle_uses_visible_polarity_and_preserves_saved_settings() {
        use ghost_shell_config::ThemeMode;
        for (mode, system_dark, expected) in [
            (ThemeMode::Dark, false, ThemeMode::Light),
            (ThemeMode::Light, true, ThemeMode::Dark),
            (ThemeMode::System, true, ThemeMode::Light),
            (ThemeMode::System, false, ThemeMode::Dark),
        ] {
            let mut shell = shell();
            shell.config.theme.mode = mode;
            shell.system_dark = system_dark;
            let saved = shell.file_config.clone();
            drop(shell.bar_message(
                Id::unique(),
                ghost_shell_bar::Message::Theme(
                    ghost_shell_widget_theme::Message::Toggle,
                ),
            ));
            assert_eq!(shell.config.theme.mode, expected);
            assert_eq!(shell.file_config, saved);
            drop(shell.bar_message(
                Id::unique(),
                ghost_shell_bar::Message::Theme(
                    ghost_shell_widget_theme::Message::Toggle,
                ),
            ));
            assert_ne!(shell.config.theme.mode, expected);
            assert_ne!(shell.config.theme.mode, ThemeMode::System);
        }
    }

    #[test]
    fn an_old_wallpaper_failure_does_not_replace_current_feedback() {
        let mut shell = shell();
        shell.wallpaper_generation = 2;
        drop(shell.update(Message::Wallpaper(1, Err("stale failure".into()))));
        assert!(shell.error.is_none());
    }
}
