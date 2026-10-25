#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WatcherEvent {
    Registered(String),
    Unregistered(String),
}
