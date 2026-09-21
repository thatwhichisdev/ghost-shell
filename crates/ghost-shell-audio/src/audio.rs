mod backend;
mod state;

use anyhow::{Result, bail};
use backend::{AudioCommand, Backend};
use ghost_shell_gpui::{
    App, AppContext as _, Context, Entity, EventEmitter, Global, Task,
};
use state::AudioUpdate;
pub use state::{
    AudioCommandError, AudioConnectionStatus, AudioDirection, AudioEndpoint,
    AudioEndpointId, AudioEvent, AudioState,
};

pub struct Audio {
    state: AudioState,
    backend: Option<Backend>,
    _events: Task<()>,
}

struct GlobalAudio(Entity<Audio>);

impl Global for GlobalAudio {}
impl EventEmitter<AudioEvent> for Audio {}

impl Audio {
    pub fn global(cx: &App) -> &Entity<Self> {
        &cx.global::<GlobalAudio>().0
    }

    pub fn state(&self) -> &AudioState {
        &self.state
    }

    /// Queues a request, not a server acknowledgement. Observe AudioEvent for results.
    pub fn set_volume(&self, endpoint: AudioEndpointId, volume: f32) -> Result<()> {
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            bail!("volume must be finite and between 0.0 and 1.0");
        }
        self.send(endpoint, AudioCommand::SetVolume(endpoint, volume))
    }

    /// Queues a request, not a server acknowledgement. Observe AudioEvent for results.
    pub fn set_muted(&self, endpoint: AudioEndpointId, muted: bool) -> Result<()> {
        self.send(endpoint, AudioCommand::SetMuted(endpoint, muted))
    }

    fn send(&self, endpoint: AudioEndpointId, command: AudioCommand) -> Result<()> {
        if self.state.connection_status() != &AudioConnectionStatus::Connected {
            bail!("audio backend is not connected");
        }
        if !self
            .state
            .endpoints()
            .contains_key(&endpoint)
        {
            bail!("audio endpoint no longer exists");
        }
        match &self.backend {
            Some(backend) => backend.send(command),
            None => bail!("audio backend is stopped"),
        }
    }

    pub fn shutdown(&mut self, cx: &mut Context<Self>) {
        drop(self.backend.take());
        self._events = Task::ready(());
        self.apply(AudioUpdate::Connection(AudioConnectionStatus::Stopped), cx);
    }

    fn apply(&mut self, update: AudioUpdate, cx: &mut Context<Self>) {
        let events = self.state.apply(update);
        if !events.is_empty() {
            for event in events {
                cx.emit(event);
            }
            cx.notify();
        }
    }
}

pub fn init(cx: &mut App) -> Entity<Audio> {
    if cx.has_global::<GlobalAudio>() {
        return Audio::global(cx).clone();
    }

    let audio = cx.new(|cx: &mut Context<Audio>| {
        let mut audio = Audio {
            state: AudioState::default(),
            backend: None,
            _events: Task::ready(()),
        };

        match Backend::start() {
            Ok((backend, mut updates)) => {
                audio.backend = Some(backend);
                audio._events = cx.spawn(async move |audio, cx| {
                    while let Some(update) = updates.recv().await {
                        if let Err(error) =
                            audio.update(cx, |audio, cx| audio.apply(update, cx))
                        {
                            log::debug!("Audio view released: {error:#}");
                            return;
                        }
                    }
                    if let Err(error) = audio.update(cx, |audio, cx| {
                        if matches!(
                            audio.state.connection_status(),
                            AudioConnectionStatus::Connecting
                                | AudioConnectionStatus::Connected
                        ) {
                            audio.apply(
                                AudioUpdate::Connection(
                                    AudioConnectionStatus::Unavailable(
                                        "audio backend stopped".into(),
                                    ),
                                ),
                                cx,
                            );
                        }
                    }) {
                        log::debug!("Audio view released: {error:#}");
                    }
                });
            }
            Err(error) => {
                log::error!("Failed to start audio backend: {error:#}");
                audio.apply(
                    AudioUpdate::Connection(AudioConnectionStatus::Unavailable(format!(
                        "{error:#}"
                    ))),
                    cx,
                );
            }
        }

        cx.on_app_quit(|audio, cx| {
            audio.shutdown(cx);
            std::future::ready(())
        })
        .detach();

        audio
    });
    cx.set_global(GlobalAudio(audio.clone()));
    audio
}
