use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    Ping,
    Shutdown,
    Bar { action: BarAction },
    Lock { action: LockAction },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LockAction {
    Lock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Handled,
}

pub type Reply = Result<Response, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BarAction {
    Open,
    Close,
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Endpoint {
    Bar,
    Lockscreen,
}

impl Endpoint {
    pub fn name(self) -> &'static str {
        match self {
            Self::Bar => "ghost-shell-bar",
            Self::Lockscreen => "ghost-shell-lockscreen",
        }
    }
}
