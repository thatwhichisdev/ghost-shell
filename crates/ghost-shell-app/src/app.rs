use std::rc::Rc;

use gpui::{Global, PlatformDisplay, accesskit::Uuid};

/// Struct that represets state of the shell
pub struct GhostShell {
    /// Collections of active shell outputs
    pub outputs: Vec<GhostShellOutput>,
}

/// Struct that represents shell output
pub struct GhostShellOutput {
    /// Reference to the platform display
    pub display: Rc<dyn PlatformDisplay>,
    /// Flag that tells if display is set as primary thru configuration
    pub is_primary: bool,
    /// Flag that tells if display is currently focused
    pub is_focused: bool,
}

impl GhostShell {
    pub fn set_focused_output(&mut self, display_uuid: Uuid) {
        for display in &mut self.outputs {
            display.is_focused = display
                .display
                .uuid()
                .is_ok_and(|uuid| uuid == display_uuid);
        }
    }

    pub fn get_output(&self) -> &GhostShellOutput {
        self.get_focused_output()
            .unwrap_or(self.get_primary_output())
    }

    pub fn get_focused_output(&self) -> Option<&GhostShellOutput> {
        self.outputs.iter().find(|display| display.is_focused)
    }

    pub fn get_primary_output(&self) -> &GhostShellOutput {
        self.outputs
            .iter()
            .find(|display| display.is_primary)
            .expect("primary output should be set")
    }
}

impl Global for GhostShell {}
