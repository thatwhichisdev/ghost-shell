use serde::{Deserialize, Serialize};

use crate::client::types::{Window, Workspace};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Event {
    WorkspacesChanged { workspaces: Vec<Workspace> },
    WorkspaceActivated { id: u64, focused: bool },
    WindowsChanged { windows: Vec<Window> },
    WindowFocusChanged { id: Option<u64> },
}
