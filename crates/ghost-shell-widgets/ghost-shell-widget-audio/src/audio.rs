use ghost_shell_audio::{Audio, AudioDirection, AudioEndpointId, AudioState};
use iced::{
    Element, Fill,
    widget::{button, column, container, row, slider, text},
};

#[derive(Debug, Clone)]
pub enum Message {
    Open,
    Dismiss,
    Volume(AudioEndpointId, f32),
    Mute(AudioEndpointId, bool),
    Default(AudioEndpointId),
}

pub fn view(audio: Option<&AudioState>) -> Element<'_, Message> {
    let audio_text = audio
        .and_then(|state| {
            state
                .default_output()
                .and_then(|id| state.endpoints().get(&id))
        })
        .map(|endpoint| {
            if endpoint.muted == Some(true) {
                "Muted".to_owned()
            } else {
                format!("{}%", (endpoint.volume.unwrap_or(0.0) * 100.0).round())
            }
        })
        .unwrap_or_else(|| "Audio".to_owned());

    let content = row![
        ghost_shell_assets::icon("icons/volume.svg", 18.0),
        text(audio_text),
    ]
    .spacing(4)
    .align_y(iced::Center);

    button(container(content).center_y(Fill))
        .padding([0, 8])
        .height(Fill)
        .on_press(Message::Open)
        .style(button::text)
        .into()
}

#[derive(Default)]
pub struct AudioPopup {
    pub error: Option<String>,
}
impl AudioPopup {
    pub const SIZE: (u32, u32) = (380, 500);

    pub fn view<'a>(&'a self, audio: Option<&'a AudioState>) -> Element<'a, Message> {
        let mut content = column![text("Audio")].spacing(10);
        if let Some(audio) = audio {
            for direction in [AudioDirection::Output, AudioDirection::Input] {
                content = content.push(text(match direction {
                    AudioDirection::Output => "Output",
                    AudioDirection::Input => "Input",
                }));
                for endpoint in audio
                    .endpoints()
                    .values()
                    .filter(|endpoint| endpoint.direction == direction)
                {
                    let id = endpoint.id;
                    let is_default = match direction {
                        AudioDirection::Output => audio.default_output(),
                        AudioDirection::Input => audio.default_input(),
                    } == Some(id);
                    let name = format!(
                        "{}{}",
                        if is_default { "✓ " } else { "" },
                        endpoint.description
                    );
                    let mut endpoint_view = column![
                        button(text(name))
                            .on_press(Message::Default(id))
                            .style(button::text)
                    ]
                    .spacing(4);
                    if let Some(volume) = endpoint.volume {
                        endpoint_view = endpoint_view.push(
                            slider(0.0..=1.0, volume, move |value| {
                                Message::Volume(id, value)
                            })
                            .step(0.01_f32),
                        );
                    }
                    if let Some(muted) = endpoint.muted {
                        endpoint_view = endpoint_view.push(
                            button(if muted { "Unmute" } else { "Mute" })
                                .on_press(Message::Mute(id, !muted)),
                        );
                    }
                    content = content.push(endpoint_view);
                }
            }
            if audio.endpoints().is_empty() {
                content = content.push(text(format!("{:?}", audio.connection_status())));
            }
        } else {
            content = content.push(text("Audio service unavailable"));
        }
        if let Some(error) = &self.error {
            content = content.push(text(error));
        }
        let content = container(iced::widget::scrollable(content))
            .padding(12)
            .width(Fill)
            .height(Fill)
            .style(|theme| ghost_shell_theme::surface(theme, 1.0));
        let (width, height) = Self::SIZE;
        // The bar owns the Wayland popup; anchor its already-open menu at the origin.
        iced_aw::DropDown::new(
            iced::widget::Space::new()
                .width(width as f32)
                .height(0),
            content,
            true,
        )
        .width(width as f32)
        .height(height as f32)
        .offset(0.0)
        .on_dismiss(Message::Dismiss)
        .into()
    }
}

impl AudioPopup {
    pub fn update(&mut self, message: Message, audio: Option<&Audio>) {
        if matches!(message, Message::Open | Message::Dismiss) {
            return;
        }
        let Some(audio) = audio else {
            self.error = Some("Audio service unavailable".into());
            return;
        };
        let result = match message {
            Message::Volume(id, volume) => audio.set_volume(id, volume),
            Message::Mute(id, muted) => audio.set_muted(id, muted),
            Message::Default(id) => audio.set_default(id),
            Message::Open | Message::Dismiss => return,
        };
        if let Err(error) = result {
            self.error = Some(error.to_string());
        }
    }
}
