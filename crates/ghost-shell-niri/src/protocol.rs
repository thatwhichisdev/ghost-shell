use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub type Reply = std::result::Result<Response, String>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Request {
    Workspaces,
    Windows,
    EventStream,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Response {
    Handled,
    Workspaces(Vec<Workspace>),
    Windows(Vec<Window>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Event {
    WorkspacesChanged { workspaces: Vec<Workspace> },
    WorkspaceActivated { id: u64, focused: bool },
    WindowsChanged { windows: Vec<Window> },
    WindowFocusChanged { id: Option<u64> },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WindowLayout {
    pub pos_in_scrolling_layout: Option<(usize, usize)>,
    pub tile_size: (f64, f64),
    pub window_size: (i32, i32),
    pub tile_pos_in_workspace_view: Option<(f64, f64)>,
    pub window_offset_in_tile: (f64, f64),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Timestamp {
    pub secs: u64,
    pub nanos: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Window {
    pub id: u64,
    pub title: Option<String>,
    pub app_id: Option<String>,
    pub pid: Option<i32>,
    pub workspace_id: Option<u64>,
    pub is_focused: bool,
    pub is_floating: bool,
    pub is_urgent: bool,
    pub layout: WindowLayout,
    pub focus_timestamp: Option<Timestamp>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: u64,
    pub idx: u8,
    pub name: Option<String>,
    pub output: Option<String>,
    pub is_urgent: bool,
    pub is_active: bool,
    pub is_focused: bool,
    pub active_window_id: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct KeyboardLayouts {
    pub names: Vec<String>,
    pub current_idx: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Cast {
    pub stream_id: u64,
    pub session_id: u64,
    pub kind: CastKind,
    pub target: CastTarget,
    pub is_dynamic_target: bool,
    pub is_active: bool,
    pub pid: Option<i32>,
    pub pw_node_id: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastKind {
    PipeWire,
    WlrScreencopy,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum CastTarget {
    Nothing {},
    Output { name: String },
    Window { id: u64 },
}

#[derive(Debug, Default, Clone)]
pub struct NiriState {
    pub workspaces: HashMap<u64, Workspace>,
    pub windows: HashMap<u64, Window>,
    pub keyboard_layouts: Option<KeyboardLayouts>,
    pub is_open: bool,
    pub failed: bool,
    pub casts: HashMap<u64, Cast>,
}

impl NiriState {
    pub fn update(&mut self, event: Event) {
        match event {
            Event::WorkspacesChanged { workspaces } => {
                self.workspaces =
                    workspaces.into_iter().map(|ws| (ws.id, ws)).collect();
            }
            Event::WorkspaceActivated { id, focused } => {
                let workspace = self
                    .workspaces
                    .get(&id)
                    .expect("activated workspace was missing from the map");
                let output = workspace.output.clone();

                for ws in self.workspaces.values_mut() {
                    let got_activated = ws.id == id;
                    if ws.output == output {
                        ws.is_active = got_activated;
                    }

                    if focused {
                        ws.is_focused = got_activated;
                    }
                }
            }
            Event::WindowsChanged { windows } => {
                self.windows =
                    windows.into_iter().map(|win| (win.id, win)).collect();
            }
            Event::WindowFocusChanged { id } => {
                for win in self.windows.values_mut() {
                    win.is_focused = Some(win.id) == id;
                }
            }
        }
    }
}
