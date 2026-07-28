use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    Launcher { action: LauncherAction },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LauncherAction {
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Handled,
}

pub type Reply = Result<Response, String>;
