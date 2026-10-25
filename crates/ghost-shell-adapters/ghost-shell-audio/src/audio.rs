pub mod pipewire;
mod state;

use std::future::Future;

use anyhow::{Result, bail};
pub use state::*;
use tokio::sync::mpsc::UnboundedReceiver;

pub trait AudioBackend: Sized {
    /// Starts a worker without waiting for device discovery; updates report availability.
    fn start() -> Result<(Self, UnboundedReceiver<AudioUpdate>)>;
    fn connect()
    -> impl Future<Output = Result<(Self, UnboundedReceiver<AudioUpdate>)>> + Send;
    /// Nonblocking queue submission. Completion/failure is delivered through AudioUpdate.
    fn enqueue(&self, command: AudioCommand) -> Result<()>;
}

pub enum AudioCommand {
    SetVolume(AudioEndpointId, f32),
    SetMuted(AudioEndpointId, bool),
    SetDefault(AudioEndpointId),
    Shutdown,
}

pub type Audio = AudioAdapter<pipewire::PipeWire>;

pub struct AudioAdapter<B: AudioBackend> {
    state: AudioState,
    backend: Option<B>,
}

impl<B: AudioBackend> AudioAdapter<B> {
    pub async fn connect() -> Result<(Self, UnboundedReceiver<AudioUpdate>)> {
        let (backend, updates) = B::connect().await?;
        Ok((
            Self {
                state: AudioState::default(),
                backend: Some(backend),
            },
            updates,
        ))
    }

    pub fn start() -> Result<(Self, tokio::sync::mpsc::UnboundedReceiver<AudioUpdate>)> {
        let (backend, updates) = B::start()?;
        Ok((
            Self {
                state: AudioState::default(),
                backend: Some(backend),
            },
            updates,
        ))
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

    /// Requests this endpoint as the default for its input or output direction.
    ///
    /// Queuing is not acknowledgement: observe `AudioEvent::DefaultsChanged` and
    /// read the effective defaults, or handle `AudioEvent::CommandFailed`.
    /// Existing stream routing remains subject to the session manager's policy.
    pub fn set_default(&self, endpoint: AudioEndpointId) -> Result<()> {
        self.send(endpoint, AudioCommand::SetDefault(endpoint))
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
            Some(backend) => backend.enqueue(command),
            None => bail!("audio backend is stopped"),
        }
    }

    pub fn apply(&mut self, update: AudioUpdate) -> Vec<AudioEvent> {
        self.state.apply(update)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    struct RecordingBackend {
        commands: RefCell<Vec<AudioCommand>>,
    }

    impl AudioBackend for RecordingBackend {
        fn start() -> Result<(Self, UnboundedReceiver<AudioUpdate>)> {
            let (updates, receiver) = tokio::sync::mpsc::unbounded_channel();
            updates.send(AudioUpdate::Connection(AudioConnectionStatus::Connected))?;
            Ok((
                Self {
                    commands: RefCell::new(Vec::new()),
                },
                receiver,
            ))
        }

        async fn connect() -> Result<(Self, UnboundedReceiver<AudioUpdate>)> {
            Self::start()
        }

        fn enqueue(&self, command: AudioCommand) -> Result<()> {
            self.commands.borrow_mut().push(command);
            Ok(())
        }
    }

    #[tokio::test]
    async fn replacement_backend_preserves_validation_and_state_ownership() -> Result<()>
    {
        let (mut audio, mut updates) =
            AudioAdapter::<RecordingBackend>::connect().await?;
        let endpoint = AudioEndpointId(7);
        assert!(audio.set_muted(endpoint, true).is_err());
        audio.apply(
            updates
                .recv()
                .await
                .ok_or_else(|| anyhow::anyhow!("missing connection update"))?,
        );
        assert!(audio.set_muted(endpoint, true).is_err());
        audio.apply(AudioUpdate::Endpoint(AudioEndpoint {
            id: endpoint,
            direction: AudioDirection::Output,
            name: "test".into(),
            description: "Test output".into(),
            volume: Some(0.5),
            muted: Some(false),
        }));
        assert!(audio.set_volume(endpoint, f32::NAN).is_err());
        assert!(audio.set_volume(endpoint, 1.1).is_err());
        audio.set_muted(endpoint, true)?;
        let backend = audio
            .backend
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("missing backend"))?;
        assert!(
            matches!(backend.commands.borrow().as_slice(), [AudioCommand::SetMuted(id, true)] if *id == endpoint)
        );
        assert_eq!(
            audio
                .state()
                .endpoints()
                .get(&endpoint)
                .and_then(|endpoint| endpoint.muted),
            Some(false)
        );
        audio.apply(AudioUpdate::Connection(AudioConnectionStatus::Stopped));
        assert!(audio.set_muted(endpoint, false).is_err());
        Ok(())
    }
}
