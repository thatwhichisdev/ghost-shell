mod menu_popup;

use ghost_shell_audio::{Audio, AudioDirection, AudioEndpointId, AudioEvent, AudioState};
use iced::{
    Element, Fill,
    widget::{button, container, row, slider, text},
};

#[derive(Debug, Clone)]
pub enum Message {
    Open(AudioDirection),
    Dismiss,
    Resize(u32, u32),
    Volume(AudioEndpointId, f32),
    Muted(AudioEndpointId, bool),
    Default(AudioEndpointId),
}

pub fn view(
    audio: Option<&AudioState>,
    style: ghost_shell_theme::bar::Style,
) -> Element<'_, Message> {
    ghost_shell_theme::bar::outer(
        row(
            [AudioDirection::Input, AudioDirection::Output].map(|direction| {
                let endpoint = audio.and_then(|state| {
                    let id = match direction {
                        AudioDirection::Input => state.default_input(),
                        AudioDirection::Output => state.default_output(),
                    }?;
                    state.endpoints().get(&id)
                });
                let muted = endpoint.is_some_and(|endpoint| endpoint.muted == Some(true));
                let icon = match direction {
                    AudioDirection::Input if muted => "icons/microphone-slash.svg",
                    AudioDirection::Input => "icons/microphone.svg",
                    AudioDirection::Output if muted => "icons/speaker-simple-slash.svg",
                    AudioDirection::Output => {
                        match endpoint.and_then(|endpoint| endpoint.volume) {
                            Some(volume) if volume <= 0.0 => {
                                "icons/speaker-simple-none.svg"
                            }
                            Some(volume) if volume < 0.5 => {
                                "icons/speaker-simple-low.svg"
                            }
                            _ => "icons/speaker-simple-high.svg",
                        }
                    }
                };
                button(ghost_shell_theme::bar::group_inner(
                    ghost_shell_theme::bar::icon(icon, style.widgets.icon_size),
                    direction == AudioDirection::Input,
                    direction == AudioDirection::Output,
                    style,
                ))
                .padding(0)
                .on_press(Message::Open(direction))
                .style(ghost_shell_theme::bar::transparent_button)
                .into()
            }),
        )
        .spacing(0)
        .align_y(iced::Center),
        style,
    )
}

pub struct AudioPopup {
    direction: AudioDirection,
    error: Option<String>,
    generation: u64,
}

impl Default for AudioPopup {
    fn default() -> Self {
        Self {
            direction: AudioDirection::Output,
            error: None,
            generation: 0,
        }
    }
}

impl AudioPopup {
    const INITIAL_WIDTH: u32 = 380;
    const MAX_WIDTH: f32 = 760.0;

    pub fn open(
        &mut self,
        direction: AudioDirection,
        audio: Option<&AudioState>,
    ) -> (u32, u32) {
        self.direction = direction;
        self.error = None;
        self.generation = self.generation.wrapping_add(1);
        let devices = audio.map_or(0, |state| {
            state
                .endpoints()
                .values()
                .filter(|endpoint| endpoint.direction == direction)
                .count()
        });
        let height = 65_usize
            .saturating_add(devices.saturating_mul(60))
            .saturating_add(devices.saturating_sub(1).saturating_mul(15))
            .min(702);
        (Self::INITIAL_WIDTH + 8, height as u32)
    }

    pub fn direction(&self) -> AudioDirection {
        self.direction
    }

    pub fn handle_event(&mut self, event: AudioEvent) {
        match event {
            AudioEvent::CommandFailed(error) => self.error = Some(error.message),
            AudioEvent::Added(_)
            | AudioEvent::Removed(_)
            | AudioEvent::ConnectionChanged => {
                // A changed device order must not transfer a live slider drag to another device.
                self.generation = self.generation.wrapping_add(1);
            }
            _ => {}
        }
    }

    pub fn view<'a>(&'a self, audio: Option<&'a AudioState>) -> Element<'a, Message> {
        use iced_aw::menu::{Item, Menu};

        let mut entries = Vec::new();
        let direction = self.direction;
        let mut found = false;
        if let Some(audio) = audio {
            let active = match direction {
                AudioDirection::Output => audio.default_output(),
                AudioDirection::Input => audio.default_input(),
            };
            for endpoint in audio
                .endpoints()
                .values()
                .filter(|endpoint| endpoint.direction == direction)
            {
                if found {
                    entries.push(
                        Item::new(
                            container(iced::widget::rule::horizontal(1))
                                .width(Fill)
                                .padding([0, 9])
                                .center_y(15),
                        )
                        .close_on_click(false),
                    );
                }
                found = true;
                let id = endpoint.id;
                let selected = active == Some(id);
                let name = button(
                    row![
                        iced::widget::Space::new()
                            .width(0)
                            .height(22),
                        text(&endpoint.description)
                    ]
                    .align_y(iced::Center),
                )
                .padding([0, 5])
                .on_press(Message::Default(id))
                .style(move |theme, status| {
                    if selected
                        || matches!(
                            status,
                            button::Status::Hovered | button::Status::Pressed
                        )
                    {
                        let mut style = ghost_shell_theme::selected(theme, status);
                        style.border.radius = 4.0.into();
                        style
                    } else {
                        ghost_shell_theme::plain(theme, status)
                    }
                });
                entries.push(
                    Item::new(
                        container(iced::widget::column![
                            name,
                            iced::widget::Space::new()
                                .width(92)
                                .height(0),
                        ])
                        .padding(4),
                    )
                    .close_on_click(false),
                );
                let volume: Element<'_, Message> = match endpoint.volume {
                    Some(volume) => {
                        slider(0.0..=1.0, volume, move |value| Message::Volume(id, value))
                            .step(0.01_f32)
                            .height(22)
                            .style(|theme: &iced::Theme, status| {
                                let mut style = slider::default(theme, status);
                                let color = theme
                                    .extended_palette()
                                    .background
                                    .neutral
                                    .color;
                                style.rail.backgrounds.0 = color.into();
                                style.handle.background = color.into();
                                style
                            })
                            .into()
                    }
                    None => text("Volume unavailable").into(),
                };
                let muted = endpoint.muted == Some(true);
                let mute = button(text("M").center())
                    .width(22)
                    .height(22)
                    .padding(0)
                    .on_press_maybe(
                        endpoint
                            .muted
                            .map(|muted| Message::Muted(id, !muted)),
                    )
                    .style(move |theme, status| {
                        if muted {
                            let mut style = ghost_shell_theme::selected(theme, status);
                            style.border.radius = 4.0.into();
                            style
                        } else {
                            ghost_shell_theme::plain(theme, status)
                        }
                    });
                entries.push(
                    Item::new(
                        container(
                            row![mute, volume]
                                .spacing(8)
                                .align_y(iced::Center),
                        )
                        .width(Fill)
                        .padding([4, 9]),
                    )
                    .close_on_click(false),
                );
            }
        }
        if !found {
            entries.push(
                Item::new(
                    container(text(match direction {
                        AudioDirection::Output => "No output devices available",
                        AudioDirection::Input => "No input devices available",
                    }))
                    .padding([4, 9]),
                )
                .close_on_click(false),
            );
        }
        if let Some(audio) = audio {
            if audio.connection_status()
                != &ghost_shell_audio::AudioConnectionStatus::Connected
            {
                entries.push(
                    Item::new(
                        container(text(format!("{:?}", audio.connection_status())))
                            .padding([4, 9])
                            .width(Fill),
                    )
                    .close_on_click(false),
                );
            }
        } else {
            entries.push(
                Item::new(
                    container(text("Audio service unavailable"))
                        .padding([4, 9])
                        .width(Fill),
                )
                .close_on_click(false),
            );
        }
        if let Some(error) = &self.error {
            entries.push(
                Item::new(
                    container(text(error).style(iced::widget::text::danger))
                        .padding([4, 9])
                        .width(Fill),
                )
                .close_on_click(false),
            );
        }
        menu_popup::Popup::new(
            Menu::new(entries)
                .width(iced::Shrink)
                .max_width(Self::MAX_WIDTH)
                .padding(4)
                .offset(8.0),
            Self::MAX_WIDTH,
            1,
            self.generation,
            Message::Resize,
            Message::Dismiss,
        )
        .into()
    }

    pub fn update(&mut self, message: Message, audio: Option<&Audio>) {
        if matches!(
            message,
            Message::Open(_) | Message::Dismiss | Message::Resize(..)
        ) {
            return;
        }
        let Some(audio) = audio else {
            self.error = Some("Audio service unavailable".into());
            return;
        };
        self.error = None;
        let result = match message {
            Message::Volume(id, volume) => audio.set_volume(id, volume),
            Message::Muted(id, muted) => audio.set_muted(id, muted),
            Message::Default(id) => audio.set_default(id),
            Message::Open(_) | Message::Dismiss | Message::Resize(..) => return,
        };
        if let Err(error) = result {
            self.error = Some(error.to_string());
        }
    }
}
