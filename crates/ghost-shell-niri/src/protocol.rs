use std::collections::{HashMap, hash_map::Entry};

use gpui::{Global, accesskit::Uuid};
use serde::{Deserialize, Serialize};

pub type Reply = std::result::Result<Response, String>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Request {
    Workspaces,
    Windows,
    EventStream,
    Action(Action),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Response {
    Handled,
    Workspaces(Vec<Workspace>),
    Windows(Vec<Window>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Event {
    WorkspacesChanged {
        workspaces: Vec<Workspace>,
    },
    WorkspaceUrgencyChanged {
        id: u64,
        urgent: bool,
    },
    WorkspaceActivated {
        id: u64,
        focused: bool,
    },
    WorkspaceActiveWindowChanged {
        workspace_id: u64,
        active_window_id: Option<u64>,
    },
    WindowsChanged {
        windows: Vec<Window>,
    },
    WindowOpenedOrChanged {
        window: Window,
    },
    WindowClosed {
        id: u64,
    },
    WindowFocusChanged {
        id: Option<u64>,
    },
    WindowFocusTimestampChanged {
        id: u64,
        focus_timestamp: Option<Timestamp>,
    },
    WindowUrgencyChanged {
        id: u64,
        urgent: bool,
    },
    WindowLayoutsChanged {
        changes: Vec<(u64, WindowLayout)>,
    },
    KeyboardLayoutsChanged {
        keyboard_layouts: KeyboardLayouts,
    },
    KeyboardLayoutSwitched {
        idx: u8,
    },
    OverviewOpenedOrClosed {
        is_open: bool,
    },
    ConfigLoaded {
        failed: bool,
    },
    ScreenshotCaptured {
        path: Option<String>,
    },
    CastsChanged {
        casts: Vec<Cast>,
    },
    CastStartedOrChanged {
        cast: Cast,
    },
    CastStopped {
        stream_id: u64,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Action {
    Spawn { command: Vec<String> },
    SpawnSh { command: String },
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
            Event::WorkspaceUrgencyChanged { id, urgent } => {
                for ws in self.workspaces.values_mut() {
                    if ws.id == id {
                        ws.is_urgent = urgent;
                    }
                }
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
            Event::WorkspaceActiveWindowChanged {
                workspace_id,
                active_window_id,
            } => {
                let ws = self.workspaces.get_mut(&workspace_id);
                let ws =
                    ws.expect("changed workspace was missing from the map");
                ws.active_window_id = active_window_id;
            }
            Event::WindowsChanged { windows } => {
                self.windows =
                    windows.into_iter().map(|win| (win.id, win)).collect();
            }
            Event::WindowOpenedOrChanged { window } => {
                let (id, is_focused) = match self.windows.entry(window.id) {
                    Entry::Occupied(mut entry) => {
                        let entry = entry.get_mut();
                        *entry = window;
                        (entry.id, entry.is_focused)
                    }
                    Entry::Vacant(entry) => {
                        let entry = entry.insert(window);
                        (entry.id, entry.is_focused)
                    }
                };

                if is_focused {
                    for win in self.windows.values_mut() {
                        if win.id != id {
                            win.is_focused = false;
                        }
                    }
                }
            }
            Event::WindowClosed { id } => {
                let win = self.windows.remove(&id);
                win.expect("closed window was missing from the map");
            }
            Event::WindowFocusChanged { id } => {
                for win in self.windows.values_mut() {
                    win.is_focused = Some(win.id) == id;
                }
            }
            Event::WindowFocusTimestampChanged {
                id,
                focus_timestamp,
            } => {
                for win in self.windows.values_mut() {
                    if win.id == id {
                        win.focus_timestamp = focus_timestamp;
                        break;
                    }
                }
            }
            Event::WindowUrgencyChanged { id, urgent } => {
                for win in self.windows.values_mut() {
                    if win.id == id {
                        win.is_urgent = urgent;
                        break;
                    }
                }
            }
            Event::WindowLayoutsChanged { changes } => {
                for (id, update) in changes {
                    let win = self.windows.get_mut(&id);
                    let win =
                        win.expect("changed window was missing from the map");
                    win.layout = update;
                }
            }
            Event::KeyboardLayoutsChanged { keyboard_layouts } => {
                self.keyboard_layouts = Some(keyboard_layouts);
            }
            Event::KeyboardLayoutSwitched { idx } => {
                let kb = self.keyboard_layouts.as_mut();
                let kb = kb.expect("keyboard layouts must be set before a layout can be switched");
                kb.current_idx = idx;
            }
            Event::OverviewOpenedOrClosed { is_open } => {
                self.is_open = is_open;
            }
            Event::ConfigLoaded { failed } => {
                self.failed = failed;
            }
            Event::CastsChanged { casts } => {
                self.casts =
                    casts.into_iter().map(|c| (c.stream_id, c)).collect();
            }
            Event::CastStartedOrChanged { cast } => {
                self.casts.insert(cast.stream_id, cast);
            }
            Event::CastStopped { stream_id } => {
                let cast = self.casts.remove(&stream_id);
                cast.expect("stopped cast was missing from the map");
            }
            _ => {}
        }
    }

    pub fn focused_output(&self) -> Option<Uuid> {
        self.workspaces
            .iter()
            .find(|(_id, workspace)| workspace.is_focused)
            .and_then(|(_id, workspace)| workspace.output.as_deref())
            .map(|output| Uuid::new_v5(&Uuid::NAMESPACE_DNS, output.as_bytes()))
    }
}

impl Global for NiriState {}
