use std::collections::HashMap;

use crate::client::{
    event::Event,
    types::{Cast, KeyboardLayouts, Window, Workspace},
};

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
                let ws = self.workspaces.get(&id);
                let ws =
                    ws.expect("activated workspace was missing from the map");
                let output = ws.output.clone();

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
