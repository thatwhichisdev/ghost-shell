use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AudioEndpointId(pub(crate) u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioDirection {
    Input,
    Output,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioEndpoint {
    pub id: AudioEndpointId,
    pub direction: AudioDirection,
    pub name: String,
    pub description: String,
    /// Cubic volume: 1.0 represents 100%. None means not yet known or unsupported.
    pub volume: Option<f32>,
    pub muted: Option<bool>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum AudioConnectionStatus {
    #[default]
    Connecting,
    Connected,
    Unavailable(String),
    Stopped,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioCommandError {
    pub endpoint: Option<AudioEndpointId>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AudioEvent {
    Added(AudioEndpoint),
    Updated(AudioEndpoint),
    Removed(AudioEndpointId),
    DefaultsChanged,
    ConnectionChanged,
    CommandFailed(AudioCommandError),
}

#[derive(Debug, Default)]
pub struct AudioState {
    endpoints: BTreeMap<AudioEndpointId, AudioEndpoint>,
    default_input: Option<AudioEndpointId>,
    default_output: Option<AudioEndpointId>,
    connection_status: AudioConnectionStatus,
    default_input_name: Option<String>,
    default_output_name: Option<String>,
}

impl AudioState {
    pub fn endpoints(&self) -> &BTreeMap<AudioEndpointId, AudioEndpoint> {
        &self.endpoints
    }

    pub fn default_input(&self) -> Option<AudioEndpointId> {
        self.default_input
    }

    pub fn default_output(&self) -> Option<AudioEndpointId> {
        self.default_output
    }

    pub fn connection_status(&self) -> &AudioConnectionStatus {
        &self.connection_status
    }

    pub(crate) fn apply(&mut self, update: AudioUpdate) -> Vec<AudioEvent> {
        let mut events = Vec::new();
        match update {
            AudioUpdate::Endpoint(endpoint) => {
                match self.endpoints.get(&endpoint.id) {
                    Some(previous) if previous == &endpoint => return events,
                    Some(_) => events.push(AudioEvent::Updated(endpoint.clone())),
                    None => events.push(AudioEvent::Added(endpoint.clone())),
                }
                self.endpoints.insert(endpoint.id, endpoint);
            }
            AudioUpdate::Removed(id) => {
                if self.endpoints.remove(&id).is_some() {
                    events.push(AudioEvent::Removed(id));
                }
            }
            AudioUpdate::Default(direction, name) => match direction {
                AudioDirection::Input => self.default_input_name = name,
                AudioDirection::Output => self.default_output_name = name,
            },
            AudioUpdate::Connection(status) => {
                if self.connection_status == status {
                    return events;
                }
                if matches!(
                    status,
                    AudioConnectionStatus::Unavailable(_)
                        | AudioConnectionStatus::Stopped
                ) {
                    events.extend(
                        self.endpoints
                            .keys()
                            .copied()
                            .map(AudioEvent::Removed),
                    );
                    self.endpoints.clear();
                    self.default_input_name = None;
                    self.default_output_name = None;
                }
                self.connection_status = status;
                events.push(AudioEvent::ConnectionChanged);
            }
            AudioUpdate::CommandFailed(error) => {
                events.push(AudioEvent::CommandFailed(error))
            }
        }

        // Metadata and nodes can arrive in either order.
        let find_default = |direction, name: &Option<String>| {
            self.endpoints
                .values()
                .find(|endpoint| {
                    endpoint.direction == direction
                        && Some(&endpoint.name) == name.as_ref()
                })
                .map(|endpoint| endpoint.id)
        };
        let input = find_default(AudioDirection::Input, &self.default_input_name);
        let output = find_default(AudioDirection::Output, &self.default_output_name);
        if input != self.default_input || output != self.default_output {
            self.default_input = input;
            self.default_output = output;
            events.push(AudioEvent::DefaultsChanged);
        }
        events
    }
}

pub(crate) enum AudioUpdate {
    Endpoint(AudioEndpoint),
    Removed(AudioEndpointId),
    Default(AudioDirection, Option<String>),
    Connection(AudioConnectionStatus),
    CommandFailed(AudioCommandError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint() -> AudioEndpoint {
        AudioEndpoint {
            id: AudioEndpointId(1),
            direction: AudioDirection::Output,
            name: "speakers".into(),
            description: "Speakers".into(),
            volume: None,
            muted: None,
        }
    }

    #[test]
    fn metadata_before_node_resolves_when_node_arrives() {
        let mut state = AudioState::default();
        assert!(
            state
                .apply(AudioUpdate::Default(
                    AudioDirection::Output,
                    Some("speakers".into())
                ))
                .is_empty()
        );
        let events = state.apply(AudioUpdate::Endpoint(endpoint()));
        assert_eq!(state.default_output(), Some(AudioEndpointId(1)));
        assert!(matches!(
            events.as_slice(),
            [AudioEvent::Added(_), AudioEvent::DefaultsChanged]
        ));
    }

    #[test]
    fn node_before_metadata_and_removal_update_default() {
        let mut state = AudioState::default();
        state.apply(AudioUpdate::Endpoint(endpoint()));
        assert_eq!(
            state.apply(AudioUpdate::Default(
                AudioDirection::Output,
                Some("speakers".into())
            )),
            vec![AudioEvent::DefaultsChanged]
        );
        assert_eq!(
            state.apply(AudioUpdate::Removed(AudioEndpointId(1))),
            vec![
                AudioEvent::Removed(AudioEndpointId(1)),
                AudioEvent::DefaultsChanged
            ]
        );
        assert_eq!(state.default_output(), None);
    }

    #[test]
    fn unchanged_updates_do_not_emit_events() {
        let mut state = AudioState::default();
        state.apply(AudioUpdate::Endpoint(endpoint()));
        assert!(
            state
                .apply(AudioUpdate::Endpoint(endpoint()))
                .is_empty()
        );
        let mut changed = endpoint();
        changed.muted = Some(true);
        assert_eq!(
            state.apply(AudioUpdate::Endpoint(changed.clone())),
            vec![AudioEvent::Updated(changed)]
        );
    }

    #[test]
    fn disconnect_clears_stale_state() {
        let mut state = AudioState::default();
        state.apply(AudioUpdate::Endpoint(endpoint()));
        state.apply(AudioUpdate::Default(
            AudioDirection::Output,
            Some("speakers".into()),
        ));
        state.apply(AudioUpdate::Connection(AudioConnectionStatus::Unavailable(
            "disconnected".into(),
        )));
        assert!(state.endpoints().is_empty());
        assert_eq!(state.default_output(), None);
        state.apply(AudioUpdate::Endpoint(endpoint()));
        assert_eq!(state.default_output(), None);
    }
}
