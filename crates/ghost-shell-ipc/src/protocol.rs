use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    Finder { action: FinderAction },
    Launcher { action: LauncherAction },
    Lock { action: LockAction },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinderAction {
    Open,
    Close,
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LauncherAction {
    Open,
    Close,
    Toggle,
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
