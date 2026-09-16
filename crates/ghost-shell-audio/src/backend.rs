use std::{
    cell::RefCell,
    collections::BTreeMap,
    io::Cursor,
    rc::Rc,
    thread::{self, JoinHandle},
};

use anyhow::{Context as _, Result, anyhow, bail};
use pipewire::{self as pw, proxy::ProxyT as _, spa};
use spa::{
    param::{ParamInfoFlags, ParamType},
    pod::{
        Object, Pod, Property, Value, ValueArray, deserialize::PodDeserializer,
        serialize::PodSerializer,
    },
    utils::dict::DictRef,
};
use tokio::sync::mpsc;

use crate::state::{
    AudioCommandError, AudioConnectionStatus, AudioDirection, AudioEndpoint,
    AudioEndpointId, AudioUpdate,
};

pub(crate) enum AudioCommand {
    SetVolume(AudioEndpointId, f32),
    SetMuted(AudioEndpointId, bool),
    Shutdown,
}

pub(crate) struct Backend {
    commands: pw::channel::Sender<AudioCommand>,
    thread: Option<JoinHandle<()>>,
}

impl Backend {
    pub(crate) fn start() -> Result<(Self, mpsc::UnboundedReceiver<AudioUpdate>)> {
        let (commands, receiver) = pw::channel::channel();
        let (updates, events) = mpsc::unbounded_channel();
        let thread = thread::Builder::new()
            .name("ghost-audio".into())
            .spawn(move || {
                if let Err(error) = run(receiver, updates.clone()) {
                    log::error!("Audio backend stopped: {error:#}");
                    if updates
                        .send(AudioUpdate::Connection(
                            AudioConnectionStatus::Unavailable(format!("{error:#}")),
                        ))
                        .is_err()
                    {
                        log::debug!("Audio event receiver already closed");
                    }
                }
            })
            .context("failed to start PipeWire thread")?;
        Ok((
            Self {
                commands,
                thread: Some(thread),
            },
            events,
        ))
    }

    pub(crate) fn send(&self, command: AudioCommand) -> Result<()> {
        if self
            .thread
            .as_ref()
            .is_none_or(JoinHandle::is_finished)
        {
            bail!("PipeWire thread has stopped");
        }
        self.commands
            .send(command)
            .map_err(|_| anyhow!("failed to send audio command"))
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            if thread.is_finished() {
                if thread.join().is_err() {
                    log::error!("PipeWire thread panicked");
                }
            } else if self
                .commands
                .send(AudioCommand::Shutdown)
                .is_err()
            {
                log::error!("Failed to request PipeWire shutdown");
            }
            // The loop is woken by Shutdown; never block GPUI waiting for a thread.
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct MixerProperties {
    channels: Vec<f32>,
    volume: Option<f32>,
    muted: Option<bool>,
}

impl MixerProperties {
    fn update(&mut self, object: &Object) {
        for property in &object.properties {
            match (property.key, &property.value) {
                (
                    spa::sys::SPA_PROP_channelVolumes,
                    Value::ValueArray(ValueArray::Float(channels)),
                ) if channels
                    .iter()
                    .all(|volume| volume.is_finite() && *volume >= 0.0) =>
                {
                    self.channels.clone_from(channels);
                }
                (spa::sys::SPA_PROP_volume, Value::Float(volume))
                    if volume.is_finite() && *volume >= 0.0 =>
                {
                    self.volume = Some(*volume);
                }
                (spa::sys::SPA_PROP_mute, Value::Bool(muted)) => {
                    self.muted = Some(*muted)
                }
                _ => {}
            }
        }
    }

    fn volume(&self) -> Option<f32> {
        self.channels
            .iter()
            .copied()
            .reduce(f32::max)
            .or(self.volume)
            .map(f32::cbrt)
    }

    fn volume_property(&self, volume: f32) -> Result<Property> {
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            bail!("volume must be finite and between 0.0 and 1.0");
        }
        let linear = volume.powi(3);
        if let Some(maximum) = self
            .channels
            .iter()
            .copied()
            .reduce(f32::max)
        {
            // Scale existing channels together to preserve balance. At silence,
            // no balance can be recovered, so initialize every channel equally.
            let channels = self
                .channels
                .iter()
                .map(|channel| {
                    if maximum > 0.0 {
                        (channel / maximum) * linear
                    } else {
                        linear
                    }
                })
                .collect();
            Ok(Property::new(
                spa::sys::SPA_PROP_channelVolumes,
                Value::ValueArray(ValueArray::Float(channels)),
            ))
        } else if self.volume.is_some() {
            Ok(Property::new(
                spa::sys::SPA_PROP_volume,
                Value::Float(linear),
            ))
        } else {
            bail!("endpoint volume is unavailable");
        }
    }
}

struct Node {
    _listener: pw::node::NodeListener,
    proxy: pw::node::Node,
    endpoint: AudioEndpoint,
    mixer: MixerProperties,
    device: Option<u32>,
    profile_device: Option<i32>,
    writable: bool,
    subscribed: bool,
}

struct Route {
    index: i32,
    device: i32,
    mixer: MixerProperties,
}

struct Device {
    _listener: pw::device::DeviceListener,
    proxy: pw::device::Device,
    routes: BTreeMap<i32, Route>,
    writable: bool,
    subscribed: bool,
}

struct Metadata {
    _listener: pw::metadata::MetadataListener,
    _proxy: pw::metadata::Metadata,
}

struct Graph {
    nodes: BTreeMap<u32, Node>,
    devices: BTreeMap<u32, Device>,
    metadata: BTreeMap<u32, Metadata>,
    next_id: u64,
    updates: mpsc::UnboundedSender<AudioUpdate>,
    main_loop: pw::main_loop::MainLoopRc,
}

impl Graph {
    fn emit(&self, update: AudioUpdate) {
        if self.updates.send(update).is_err() {
            log::debug!("Audio event receiver closed; stopping PipeWire");
            self.main_loop.quit();
        }
    }

    fn failed(&self, endpoint: Option<AudioEndpointId>, error: impl std::fmt::Display) {
        log::warn!("Audio operation failed: {error}");
        self.emit(AudioUpdate::CommandFailed(AudioCommandError {
            endpoint,
            message: error.to_string(),
        }));
    }

    fn route(&self, node: &Node) -> Option<(&Device, &Route)> {
        let device = self.devices.get(&node.device?)?;
        let route = device.routes.get(&node.profile_device?)?;
        Some((device, route))
    }

    fn publish(&self, global_id: u32) {
        if let Some(node) = self.nodes.get(&global_id) {
            let mixer = self
                .route(node)
                .map_or(&node.mixer, |(_, route)| &route.mixer);
            let mut endpoint = node.endpoint.clone();
            endpoint.volume = mixer.volume();
            endpoint.muted = mixer.muted;
            self.emit(AudioUpdate::Endpoint(endpoint));
        }
    }

    fn publish_device(&self, device: u32) {
        for (global_id, node) in &self.nodes {
            if node.device == Some(device) {
                self.publish(*global_id);
            }
        }
    }

    fn command(&self, command: AudioCommand) {
        let (endpoint, volume, muted) = match command {
            AudioCommand::SetVolume(endpoint, volume) => (endpoint, Some(volume), None),
            AudioCommand::SetMuted(endpoint, muted) => (endpoint, None, Some(muted)),
            AudioCommand::Shutdown => {
                self.main_loop.quit();
                return;
            }
        };
        if let Err(error) = self.set_parameter(endpoint, volume, muted) {
            self.failed(Some(endpoint), error);
        }
    }

    fn set_parameter(
        &self,
        endpoint: AudioEndpointId,
        volume: Option<f32>,
        muted: Option<bool>,
    ) -> Result<()> {
        let node = self
            .nodes
            .values()
            .find(|node| node.endpoint.id == endpoint)
            .context("audio endpoint was removed")?;
        let route = self.route(node);
        let mixer = route.map_or(&node.mixer, |(_, route)| &route.mixer);
        let property = if let Some(volume) = volume {
            mixer.volume_property(volume)?
        } else if let Some(muted) = muted {
            if mixer.muted.is_none() {
                bail!("endpoint mute is unavailable");
            }
            Property::new(spa::sys::SPA_PROP_mute, Value::Bool(muted))
        } else {
            bail!("missing audio parameter");
        };
        let props = Value::Object(Object {
            type_: spa::sys::SPA_TYPE_OBJECT_Props,
            id: ParamType::Props.as_raw(),
            properties: vec![property],
        });
        let (parameter, value) = match route {
            Some((device, route)) => {
                if !device.writable {
                    bail!("device route is read-only");
                }
                (
                    ParamType::Route,
                    Value::Object(Object {
                        type_: spa::sys::SPA_TYPE_OBJECT_ParamRoute,
                        id: ParamType::Route.as_raw(),
                        properties: vec![
                            Property::new(
                                spa::sys::SPA_PARAM_ROUTE_index,
                                Value::Int(route.index),
                            ),
                            Property::new(
                                spa::sys::SPA_PARAM_ROUTE_device,
                                Value::Int(route.device),
                            ),
                            Property::new(spa::sys::SPA_PARAM_ROUTE_props, props),
                            Property::new(
                                spa::sys::SPA_PARAM_ROUTE_save,
                                Value::Bool(true),
                            ),
                        ],
                    }),
                )
            }
            None => {
                if !node.writable {
                    bail!("audio node is read-only");
                }
                (ParamType::Props, props)
            }
        };
        let (bytes, _) = PodSerializer::serialize(Cursor::new(Vec::new()), &value)
            .map_err(|error| anyhow!("failed to encode audio parameter: {error:?}"))?;
        let bytes = bytes.into_inner();
        let pod = Pod::from_bytes(&bytes).context("invalid encoded audio parameter")?;
        if let Some((device, _)) = route {
            device.proxy.set_param(parameter, 0, pod);
        } else {
            node.proxy.set_param(parameter, 0, pod);
        }
        Ok(())
    }

    fn remove(&mut self, global_id: u32) {
        if let Some(node) = self.nodes.remove(&global_id) {
            self.emit(AudioUpdate::Removed(node.endpoint.id));
        }
        if self.devices.remove(&global_id).is_some() {
            self.publish_device(global_id);
        }
        if self.metadata.remove(&global_id).is_some() {
            self.emit(AudioUpdate::Default(AudioDirection::Input, None));
            self.emit(AudioUpdate::Default(AudioDirection::Output, None));
        }
    }
}

fn run(
    commands: pw::channel::Receiver<AudioCommand>,
    updates: mpsc::UnboundedSender<AudioUpdate>,
) -> Result<()> {
    let main_loop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&main_loop, None)?;
    let core = context
        .connect_rc(None)
        .context("failed to connect to PipeWire")?;
    let registry = core.get_registry_rc()?;
    let graph = Rc::new(RefCell::new(Graph {
        nodes: BTreeMap::new(),
        devices: BTreeMap::new(),
        metadata: BTreeMap::new(),
        next_id: 0,
        updates,
        main_loop: main_loop.clone(),
    }));

    let _core_listener = core
        .add_listener_local()
        .info({
            let graph = Rc::downgrade(&graph);
            move |_| {
                if let Some(graph) = graph.upgrade() {
                    graph
                        .borrow()
                        .emit(AudioUpdate::Connection(AudioConnectionStatus::Connected));
                }
            }
        })
        .error({
            let graph = Rc::downgrade(&graph);
            move |object, _sequence, result, message| {
                if let Some(graph) = graph.upgrade() {
                    let graph = graph.borrow();
                    let message =
                        format!("PipeWire object {object}: {message} ({result})");
                    // The core also reports benign errors when a global disappears
                    // between discovery and binding. Only a broken connection ends
                    // the service (the same distinction used by PipeWire's pw-mon).
                    let error_kind = result
                        .checked_neg()
                        .map(|code| std::io::Error::from_raw_os_error(code).kind());
                    if object == pw::core::PW_ID_CORE
                        && error_kind == Some(std::io::ErrorKind::BrokenPipe)
                    {
                        graph.emit(AudioUpdate::Connection(
                            AudioConnectionStatus::Unavailable(message),
                        ));
                        graph.main_loop.quit();
                    } else if error_kind == Some(std::io::ErrorKind::NotFound) {
                        log::debug!("Audio object disappeared: {message}");
                    } else {
                        let endpoint = graph
                            .nodes
                            .values()
                            .find(|node| node.proxy.upcast_ref().id() == object)
                            .map(|node| node.endpoint.id);
                        graph.failed(endpoint, message);
                    }
                }
            }
        })
        .register();

    let _registry_listener = registry
        .add_listener_local()
        .global({
            let registry = registry.downgrade();
            let graph = Rc::downgrade(&graph);
            move |global| {
                let (Some(registry), Some(graph)) = (registry.upgrade(), graph.upgrade())
                else {
                    return;
                };
                let result = match global.type_ {
                    pw::types::ObjectType::Node => bind_node(&registry, global, &graph),
                    pw::types::ObjectType::Device => {
                        bind_device(&registry, global, &graph)
                    }
                    pw::types::ObjectType::Metadata => {
                        bind_metadata(&registry, global, &graph)
                    }
                    _ => Ok(()),
                };
                if let Err(error) = result {
                    graph.borrow().failed(None, error);
                }
            }
        })
        .global_remove({
            let graph = Rc::downgrade(&graph);
            move |global_id| {
                if let Some(graph) = graph.upgrade() {
                    graph.borrow_mut().remove(global_id);
                }
            }
        })
        .register();

    let _commands = commands.attach(main_loop.loop_(), {
        let graph = Rc::downgrade(&graph);
        move |command| {
            if let Some(graph) = graph.upgrade() {
                graph.borrow().command(command);
            }
        }
    });
    main_loop.run();
    Ok(())
}

fn direction(properties: &DictRef) -> Option<AudioDirection> {
    match properties.get("media.class")? {
        "Audio/Sink" => Some(AudioDirection::Output),
        "Audio/Source" | "Audio/Source/Virtual" => Some(AudioDirection::Input),
        _ => None,
    }
}

fn bind_node<P: AsRef<DictRef>>(
    registry: &pw::registry::Registry,
    global: &pw::registry::GlobalObject<P>,
    graph: &Rc<RefCell<Graph>>,
) -> Result<()> {
    let Some(properties) = global.props.as_ref().map(AsRef::as_ref) else {
        return Ok(());
    };
    let Some(direction) = direction(properties) else {
        return Ok(());
    };
    let global_id = global.id;
    let proxy: pw::node::Node = registry.bind(global)?;
    let listener = proxy
        .add_listener_local()
        .info({
            let graph = Rc::downgrade(graph);
            move |info| {
                let Some(graph) = graph.upgrade() else {
                    return;
                };
                let mut graph = graph.borrow_mut();
                if let Some(node) = graph.nodes.get_mut(&global_id) {
                    if let Some(properties) = info.props() {
                        if let Some(name) = properties.get("node.name") {
                            node.endpoint.name = name.into();
                        }
                        if let Some(description) = properties
                            .get("node.description")
                            .or_else(|| properties.get("node.nick"))
                        {
                            node.endpoint.description = description.into();
                        }
                        if let Some(device) = properties.get("device.id") {
                            node.device = device.parse().ok();
                        }
                        if let Some(profile_device) =
                            properties.get("card.profile.device")
                        {
                            node.profile_device = profile_device.parse().ok();
                        }
                    }
                    if let Some(parameter) = info
                        .params()
                        .iter()
                        .find(|parameter| parameter.id() == ParamType::Props)
                    {
                        node.writable = parameter
                            .flags()
                            .contains(ParamInfoFlags::WRITE);
                        if !node.subscribed
                            && parameter
                                .flags()
                                .contains(ParamInfoFlags::READ)
                        {
                            node.proxy
                                .subscribe_params(&[ParamType::Props]);
                            node.proxy.enum_params(
                                0,
                                Some(ParamType::Props),
                                0,
                                u32::MAX,
                            );
                            node.subscribed = true;
                        }
                    }
                }
                graph.publish(global_id);
            }
        })
        .param({
            let graph = Rc::downgrade(graph);
            move |_sequence, parameter, _index, _next, pod| {
                if parameter != ParamType::Props {
                    return;
                }
                let (Some(graph), Some(pod)) = (graph.upgrade(), pod) else {
                    return;
                };
                let mut graph = graph.borrow_mut();
                match decode_object(pod) {
                    Ok(object) => {
                        if let Some(node) = graph.nodes.get_mut(&global_id) {
                            node.mixer.update(&object);
                        }
                        graph.publish(global_id);
                    }
                    Err(error) => graph.failed(None, error),
                }
            }
        })
        .register();

    let mut graph = graph.borrow_mut();
    graph.next_id = graph
        .next_id
        .checked_add(1)
        .context("audio endpoint identifier exhausted")?;
    let name = properties
        .get("node.name")
        .unwrap_or_default()
        .to_owned();
    let endpoint = AudioEndpoint {
        id: AudioEndpointId(graph.next_id),
        direction,
        description: properties
            .get("node.description")
            .or_else(|| properties.get("node.nick"))
            .unwrap_or(&name)
            .to_owned(),
        name,
        volume: None,
        muted: None,
    };
    graph.nodes.insert(
        global_id,
        Node {
            _listener: listener,
            proxy,
            endpoint,
            mixer: MixerProperties::default(),
            device: properties
                .get("device.id")
                .and_then(|value| value.parse().ok()),
            profile_device: properties
                .get("card.profile.device")
                .and_then(|value| value.parse().ok()),
            writable: false,
            subscribed: false,
        },
    );
    graph.publish(global_id);
    Ok(())
}

fn bind_device<P: AsRef<DictRef>>(
    registry: &pw::registry::Registry,
    global: &pw::registry::GlobalObject<P>,
    graph: &Rc<RefCell<Graph>>,
) -> Result<()> {
    if global
        .props
        .as_ref()
        .and_then(|properties| properties.as_ref().get("media.class"))
        != Some("Audio/Device")
    {
        return Ok(());
    }
    let global_id = global.id;
    let proxy: pw::device::Device = registry.bind(global)?;
    let listener = proxy
        .add_listener_local()
        .info({
            let graph = Rc::downgrade(graph);
            move |info| {
                let Some(graph) = graph.upgrade() else {
                    return;
                };
                let mut graph = graph.borrow_mut();
                if let Some(device) = graph.devices.get_mut(&global_id)
                    && let Some(parameter) = info
                        .params()
                        .iter()
                        .find(|parameter| parameter.id() == ParamType::Route)
                {
                    device.writable = parameter
                        .flags()
                        .contains(ParamInfoFlags::WRITE);
                    if parameter
                        .flags()
                        .contains(ParamInfoFlags::READ)
                    {
                        // Route sets can shrink on profile changes. Re-enumerate
                        // on parameter-info changes so stale routes do not survive.
                        device.routes.clear();
                        if !device.subscribed {
                            device
                                .proxy
                                .subscribe_params(&[ParamType::Route]);
                            device.subscribed = true;
                        }
                        device
                            .proxy
                            .enum_params(0, Some(ParamType::Route), 0, u32::MAX);
                    }
                }
                graph.publish_device(global_id);
            }
        })
        .param({
            let graph = Rc::downgrade(graph);
            move |_sequence, parameter, _index, _next, pod| {
                if parameter != ParamType::Route {
                    return;
                }
                let (Some(graph), Some(pod)) = (graph.upgrade(), pod) else {
                    return;
                };
                let mut graph = graph.borrow_mut();
                match decode_object(pod).and_then(|object| decode_route(&object)) {
                    Ok(Some(route)) => {
                        if let Some(device) = graph.devices.get_mut(&global_id) {
                            device.routes.insert(route.device, route);
                        }
                        graph.publish_device(global_id);
                    }
                    Ok(None) => {}
                    Err(error) => graph.failed(None, error),
                }
            }
        })
        .register();
    graph.borrow_mut().devices.insert(
        global_id,
        Device {
            _listener: listener,
            proxy,
            routes: BTreeMap::new(),
            writable: false,
            subscribed: false,
        },
    );
    Ok(())
}

fn bind_metadata<P: AsRef<DictRef>>(
    registry: &pw::registry::Registry,
    global: &pw::registry::GlobalObject<P>,
    graph: &Rc<RefCell<Graph>>,
) -> Result<()> {
    if global
        .props
        .as_ref()
        .and_then(|properties| properties.as_ref().get("metadata.name"))
        != Some("default")
    {
        return Ok(());
    }
    let proxy: pw::metadata::Metadata = registry.bind(global)?;
    let listener = proxy
        .add_listener_local()
        .property({
            let graph = Rc::downgrade(graph);
            move |subject, key, _type, value| {
                if subject != pw::core::PW_ID_CORE {
                    return 0;
                }
                let Some(graph) = graph.upgrade() else {
                    return 0;
                };
                let graph = graph.borrow();
                let direction = match key {
                    Some("default.audio.sink") => AudioDirection::Output,
                    Some("default.audio.source") => AudioDirection::Input,
                    None => {
                        graph.emit(AudioUpdate::Default(AudioDirection::Input, None));
                        graph.emit(AudioUpdate::Default(AudioDirection::Output, None));
                        return 0;
                    }
                    _ => return 0,
                };
                match default_name(value) {
                    Ok(name) => graph.emit(AudioUpdate::Default(direction, name)),
                    Err(error) => {
                        graph.emit(AudioUpdate::Default(direction, None));
                        graph.failed(None, error);
                    }
                }
                0
            }
        })
        .register();
    graph.borrow_mut().metadata.insert(
        global.id,
        Metadata {
            _listener: listener,
            _proxy: proxy,
        },
    );
    Ok(())
}

fn default_name(value: Option<&str>) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value: serde_json::Value =
        serde_json::from_str(value).context("invalid default audio metadata")?;
    let name = value
        .get("name")
        .and_then(serde_json::Value::as_str)
        .context("default audio metadata has no name")?;
    Ok(Some(name.to_owned()))
}

fn decode_object(pod: &Pod) -> Result<Object> {
    match PodDeserializer::deserialize_any_from(pod.as_bytes()) {
        Ok((_, Value::Object(object))) => Ok(object),
        Ok(_) => bail!("expected audio parameter object"),
        Err(error) => bail!("invalid audio parameter: {error:?}"),
    }
}

fn decode_route(object: &Object) -> Result<Option<Route>> {
    let mut index = None;
    let mut device = None;
    let mut mixer = None;
    for property in &object.properties {
        match (property.key, &property.value) {
            (spa::sys::SPA_PARAM_ROUTE_index, Value::Int(value)) => index = Some(*value),
            (spa::sys::SPA_PARAM_ROUTE_device, Value::Int(value)) => {
                device = Some(*value)
            }
            (spa::sys::SPA_PARAM_ROUTE_props, Value::Object(props)) => {
                let mut properties = MixerProperties::default();
                properties.update(props);
                mixer = Some(properties);
            }
            _ => {}
        }
    }
    let Some(mixer) = mixer else {
        return Ok(None);
    };
    Ok(Some(Route {
        index: index.context("audio route has no index")?,
        device: device.context("audio route has no profile device")?,
        mixer,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_parameters_round_trip() -> Result<()> {
        let route = Value::Object(Object {
            type_: spa::sys::SPA_TYPE_OBJECT_ParamRoute,
            id: ParamType::Route.as_raw(),
            properties: vec![
                Property::new(spa::sys::SPA_PARAM_ROUTE_index, Value::Int(3)),
                Property::new(spa::sys::SPA_PARAM_ROUTE_device, Value::Int(1)),
                Property::new(
                    spa::sys::SPA_PARAM_ROUTE_props,
                    Value::Object(Object {
                        type_: spa::sys::SPA_TYPE_OBJECT_Props,
                        id: ParamType::Props.as_raw(),
                        properties: vec![
                            Property::new(spa::sys::SPA_PROP_mute, Value::Bool(true)),
                            Property::new(
                                spa::sys::SPA_PROP_channelVolumes,
                                Value::ValueArray(ValueArray::Float(vec![0.125, 0.125])),
                            ),
                        ],
                    }),
                ),
            ],
        });
        let (bytes, _) = PodSerializer::serialize(Cursor::new(Vec::new()), &route)
            .map_err(|error| anyhow!("{error:?}"))?;
        let bytes = bytes.into_inner();
        let pod = Pod::from_bytes(&bytes).context("invalid test parameter")?;
        let route = decode_route(&decode_object(pod)?)?.context("missing route")?;
        assert_eq!((route.index, route.device), (3, 1));
        assert_eq!(route.mixer.volume(), Some(0.5));
        assert_eq!(route.mixer.muted, Some(true));
        Ok(())
    }

    #[test]
    #[ignore = "requires a running PipeWire session; reads state without changing audio"]
    fn session_discovery_and_shutdown() -> Result<()> {
        use std::time::{Duration, Instant};

        let (mut backend, mut updates) = Backend::start()?;
        let mut state = crate::AudioState::default();
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            while let Ok(update) = updates.try_recv() {
                state.apply(update);
            }
            if let AudioConnectionStatus::Unavailable(message) = state.connection_status()
            {
                bail!("{message}");
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(state.connection_status(), &AudioConnectionStatus::Connected);
        eprintln!(
            "Discovered {} audio endpoints, {} with volume, {} with mute",
            state.endpoints().len(),
            state
                .endpoints()
                .values()
                .filter(|endpoint| endpoint.volume.is_some())
                .count(),
            state
                .endpoints()
                .values()
                .filter(|endpoint| endpoint.muted.is_some())
                .count()
        );
        backend.send(AudioCommand::Shutdown)?;
        let deadline = Instant::now() + Duration::from_secs(3);
        while backend
            .thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
        {
            if Instant::now() >= deadline {
                bail!("PipeWire thread did not stop");
            }
            thread::sleep(Duration::from_millis(10));
        }
        if let Some(thread) = backend.thread.take() {
            thread
                .join()
                .map_err(|_| anyhow!("PipeWire thread panicked"))?;
        }
        Ok(())
    }

    #[test]
    fn volume_conversion_preserves_balance() -> Result<()> {
        let mixer = MixerProperties {
            channels: vec![1.0, 0.5],
            ..Default::default()
        };
        assert_eq!(mixer.volume(), Some(1.0));
        let property = mixer.volume_property(0.5)?;
        assert_eq!(
            property.value,
            Value::ValueArray(ValueArray::Float(vec![0.125, 0.0625]))
        );
        Ok(())
    }

    #[test]
    fn silent_channels_can_be_raised() -> Result<()> {
        let mixer = MixerProperties {
            channels: vec![0.0, 0.0],
            ..Default::default()
        };
        assert_eq!(
            mixer.volume_property(1.0)?.value,
            Value::ValueArray(ValueArray::Float(vec![1.0, 1.0]))
        );
        Ok(())
    }

    #[test]
    fn rejects_invalid_volume_and_unknown_controls() {
        let mixer = MixerProperties {
            volume: Some(1.0),
            ..Default::default()
        };
        for volume in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            assert!(mixer.volume_property(volume).is_err());
        }
        assert!(
            MixerProperties::default()
                .volume_property(0.5)
                .is_err()
        );
    }

    #[test]
    fn partial_properties_preserve_other_controls() {
        let mut mixer = MixerProperties {
            channels: vec![0.125],
            muted: Some(false),
            volume: None,
        };
        mixer.update(&Object {
            type_: spa::sys::SPA_TYPE_OBJECT_Props,
            id: ParamType::Props.as_raw(),
            properties: vec![Property::new(spa::sys::SPA_PROP_mute, Value::Bool(true))],
        });
        assert_eq!(mixer.volume(), Some(0.5));
        assert_eq!(mixer.muted, Some(true));
    }

    #[test]
    fn parses_and_clears_defaults() -> Result<()> {
        assert_eq!(
            default_name(Some(r#"{"name":"alsa_output.test"}"#))?,
            Some("alsa_output.test".into())
        );
        assert_eq!(default_name(None)?, None);
        assert!(default_name(Some("not json")).is_err());
        assert!(default_name(Some("{}")).is_err());
        Ok(())
    }
}
