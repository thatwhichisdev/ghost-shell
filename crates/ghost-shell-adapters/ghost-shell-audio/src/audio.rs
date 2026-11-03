pub mod pipewire;
mod state;

use std::future::Future;

pub use state::*;
use tokio::sync::mpsc::UnboundedReceiver;

pub type Result<T> = std::result::Result<T, Error>;

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
            return InvalidVolumeSnafu.fail();
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
            return DisconnectedSnafu.fail();
        }
        if !self
            .state
            .endpoints()
            .contains_key(&endpoint)
        {
            return MissingEndpointSnafu.fail();
        }
        match &self.backend {
            Some(backend) => backend.enqueue(command),
            None => StoppedSnafu.fail(),
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
            assert!(
                updates
                    .send(AudioUpdate::Connection(AudioConnectionStatus::Connected))
                    .is_ok()
            );
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

    #[test]
    fn default_requests_and_failures_do_not_change_the_confirmed_device()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let (mut audio, _) = AudioAdapter::<RecordingBackend>::start()?;
        audio.apply(AudioUpdate::Connection(AudioConnectionStatus::Connected));
        for (number, name) in [(1, "speakers"), (2, "headphones")] {
            audio.apply(AudioUpdate::Endpoint(AudioEndpoint {
                id: AudioEndpointId(number),
                direction: AudioDirection::Output,
                name: name.into(),
                description: name.into(),
                volume: Some(0.5),
                muted: Some(false),
            }));
        }
        audio.apply(AudioUpdate::Default(
            AudioDirection::Output,
            Some("speakers".into()),
        ));
        audio.set_default(AudioEndpointId(2))?;
        assert_eq!(audio.state().default_output(), Some(AudioEndpointId(1)));
        audio.apply(AudioUpdate::CommandFailed(AudioCommandError {
            endpoint: Some(AudioEndpointId(2)),
            message: "activation rejected".into(),
        }));
        assert_eq!(audio.state().default_output(), Some(AudioEndpointId(1)));
        audio.apply(AudioUpdate::Default(
            AudioDirection::Output,
            Some("headphones".into()),
        ));
        assert_eq!(audio.state().default_output(), Some(AudioEndpointId(2)));
        Ok(())
    }

    #[tokio::test]
    async fn replacement_backend_preserves_validation_and_state_ownership()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let (mut audio, mut updates) =
            AudioAdapter::<RecordingBackend>::connect().await?;
        let endpoint = AudioEndpointId(7);
        assert!(audio.set_muted(endpoint, true).is_err());
        audio.apply(
            updates
                .recv()
                .await
                .ok_or("missing connection update")?,
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
        assert!(matches!(
            audio.set_volume(endpoint, f32::NAN),
            Err(Error::InvalidVolume)
        ));
        assert!(matches!(
            audio.set_volume(endpoint, 1.1),
            Err(Error::InvalidVolume)
        ));
        audio.set_muted(endpoint, true)?;
        let backend = audio
            .backend
            .as_ref()
            .ok_or("missing backend")?;
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

#[derive(Debug, snafu::Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("volume must be finite and between 0.0 and 1.0"))]
    InvalidVolume,
    #[snafu(display("audio backend is not connected"))]
    Disconnected,
    #[snafu(display("audio endpoint no longer exists"))]
    MissingEndpoint,
    #[snafu(display("audio backend is stopped"))]
    Stopped,
    #[snafu(display("PipeWire thread has stopped"))]
    WorkerStopped,
    #[snafu(display("endpoint volume is unavailable"))]
    VolumeUnavailable,
    #[snafu(display("audio endpoint was removed"))]
    RemovedEndpoint,
    #[snafu(display("audio endpoint has no node name"))]
    MissingNodeName,
    #[snafu(display("default audio metadata is unavailable or read-only"))]
    MetadataUnavailable,
    #[snafu(display("endpoint mute is unavailable"))]
    MuteUnavailable,
    #[snafu(display("missing audio parameter"))]
    MissingParameter,
    #[snafu(display("device route is read-only"))]
    ReadOnlyRoute,
    #[snafu(display("audio node is read-only"))]
    ReadOnlyNode,
    #[snafu(display("invalid encoded audio parameter"))]
    InvalidEncodedParameter,
    #[snafu(display("audio endpoint identifier exhausted"))]
    IdentifiersExhausted,
    #[snafu(display("default audio metadata has no name"))]
    MissingDefaultName,
    #[snafu(display("expected audio parameter object"))]
    ExpectedObject,
    #[snafu(display("audio route has no index"))]
    MissingRouteIndex,
    #[snafu(display("audio route has no profile device"))]
    MissingProfileDevice,
    #[snafu(display("Audio command queue closed"))]
    CommandQueueClosed,
    #[snafu(display("Could not start PipeWire worker"))]
    StartWorker { source: std::io::Error },
    #[snafu(display("Could not connect to PipeWire"))]
    Connect { source: pipewire::pw::Error },
    #[snafu(display("Could not create PipeWire main loop"))]
    MainLoop { source: pipewire::pw::Error },
    #[snafu(display("Could not create PipeWire context"))]
    Context { source: pipewire::pw::Error },
    #[snafu(display("Could not acquire PipeWire registry"))]
    Registry { source: pipewire::pw::Error },
    #[snafu(display("Could not bind PipeWire object {id}"))]
    Bind {
        id: u32,
        source: pipewire::pw::Error,
    },
    #[snafu(display("Could not decode default audio metadata"))]
    DefaultMetadata { source: serde_json::Error },
    #[snafu(display("Could not encode audio parameter"))]
    EncodeParameter {
        source: pipewire::pw::spa::pod::serialize::GenError,
    },
    #[snafu(display("Invalid audio parameter: {detail}"))]
    InvalidParameter { detail: String },
}
