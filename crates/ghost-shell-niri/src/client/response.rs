use serde::{Deserialize, Serialize};

use crate::client::types::{Window, Workspace};

pub type Reply = std::result::Result<Response, String>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Response {
    Handled,
    Workspaces(Vec<Workspace>),
    Windows(Vec<Window>),
}
