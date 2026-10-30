use std::collections::{BTreeMap, HashMap};

use futures::SinkExt as _;
use ghost_shell_bar::Bar;
use ghost_shell_config::{BarAppConfig, BarAppearance, BarConfig};
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
    Niri(
        Result<
            ghost_shell_niri::NiriEvent,
            std::sync::Arc<ghost_shell_niri::StreamError>,
        >,
    ),
    Bar(Id, ghost_shell_bar::Message),
    Wallpaper(Result<WallpaperSource, String>),
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
}

enum PopupRequest {
    Audio(Id, ghost_shell_audio::AudioDirection),
    Menu(Id, ghost_shell_dbus::MenuId),
}

struct Shell {
    config: BarAppConfig,
    default_bar: BarConfig,
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
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config =
        ghost_shell_config::load::<BarAppConfig>("bar").unwrap_or_else(|error| {
            log::warn!("Configuration: {error}");

            BarAppConfig::default()
        });

    if config.bars.is_empty() {
        log::info!("No bars configured; showing a default bar on each output");
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
        move || Shell::new(config.clone(), events.clone()),
        "ghost-shell-bar",
        Shell::update,
        Shell::view,
    )
    .subscription(Shell::subscription)
    .theme(|shell: &Shell, _window| {
        ghost_shell_theme::theme(&shell.config.theme, shell.system_dark)
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
                Message::Wallpaper,
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
            tray: ghost_shell_widget_tray::Tray::new(config.general.theme.clone()),
            ..Bar::default()
        };
        let mut shell = Self {
            config,
            default_bar: BarConfig::default(),
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
        };

        shell.clock();

        (shell, Task::batch(tasks))
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.events.listen().map(Message::Shell),
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
        if self.config.bars.is_empty() {
            Some(&self.default_bar)
        } else {
            self.config.bars.get(output)
        }
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
                            blur_option: if config.appearance == BarAppearance::Blur {
                                BlurOption::FullRegion
                            } else {
                                BlurOption::None
                            },
                            ..Default::default()
                        },
                    ));
                }
            }
        }

        Task::batch(tasks)
    }

    fn close(&mut self, window: Id) -> Task<Message> {
        if matches!(
            self.surfaces.get(&window),
            Some(Surface::Audio | Surface::Menu)
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

        let settings = IcedNewPopupSettings::new(
            parent,
            PixelSize::px(width, height),
            Position::new(position.x as i32, position.y as i32),
            PixelSize::px(1, 1),
        )
        .anchor(PopupAnchor::BottomRight)
        .gravity(PopupGravity::BottomLeft)
        .constraint_adjustment(
            PopupConstraintAdjustment::SlideX | PopupConstraintAdjustment::SlideY,
        );

        self.popup_settings.insert(id, settings);

        (id, Task::done(Message::NewPopUp { id, settings }))
    }

    fn request_popup(&mut self, request: PopupRequest) -> Task<Message> {
        self.cancel_loading_menu();
        let parent = match &request {
            PopupRequest::Audio(parent, _) | PopupRequest::Menu(parent, _) => *parent,
        };
        if !self.bars_visible
            || !matches!(self.surfaces.get(&parent), Some(Surface::Bar(_)))
        {
            return Task::none();
        }
        if let Some(window) = self
            .find(Surface::Menu)
            .or_else(|| self.find(Surface::Audio))
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
                {
                    if let Some(request) = self.pending_popup.take() {
                        return self.request_popup(request);
                    }
                }
            }
            Message::Shell(_) => {}
            Message::Wallpaper(Ok(source)) => self.wallpaper = source,
            Message::Wallpaper(Err(error)) => self.error(format!("Wallpaper: {error}")),
            Message::Niri(update) => match update {
                Ok(event) => self.niri.update(event),
                Err(error) => {
                    self.niri = Default::default();
                    self.error(format!("Niri: {error}"));
                }
            },
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
                    Some(Surface::Audio | Surface::Menu)
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
            BarMessage::Menu(ghost_shell_widget_menu::Message::OpenLauncher) => {
                return ghost_shell_niri::send(ghost_shell_niri::Request::Action(
                    ghost_shell_niri::Action::Spawn {
                        command: vec!["ghost-shell-launcher".into()],
                    },
                ))
                .map(|result| {
                    Message::CommandResult(result.map_err(|error| error.to_string()))
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

        if key == Named::Escape && matches!(surface, Some(Surface::Audio | Surface::Menu))
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
                        name,
                        &self.niri,
                        self.audio
                            .as_ref()
                            .map(|audio| audio.state()),
                    )
                    .map(move |message| Message::Bar(window, message)),
                None => text("").into(),
            },
            Some(Surface::Audio) => self
                .audio_popup
                .view(
                    self.audio
                        .as_ref()
                        .map(|audio| audio.state()),
                )
                .map(move |message| {
                    Message::Bar(window, ghost_shell_bar::Message::Audio(message))
                }),
            Some(Surface::Menu) => match &self.menu_state {
                Some(state) => state.view().map(move |message| {
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
