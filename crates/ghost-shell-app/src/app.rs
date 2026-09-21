use std::rc::Rc;

use ghost_shell_gpui::{Global, PlatformDisplay, accesskit::Uuid};

/// Struct that represets state of the shell
pub struct GhostShell {
    /// Active shell outputs
    outputs: Vec<GhostShellOutput>,
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
    pub fn new(outputs: Vec<GhostShellOutput>) -> Self {
        Self { outputs }
    }

    pub fn set_focused_output(&mut self, display_uuid: Option<Uuid>) {
        for display in &mut self.outputs {
            display.is_focused = display_uuid.is_some_and(|focused| {
                display
                    .display
                    .uuid()
                    .is_ok_and(|uuid| uuid == focused)
            });
        }
    }

    pub(crate) fn set_displays(
        &mut self,
        mut displays: Vec<Rc<dyn PlatformDisplay>>,
        primary: Option<Uuid>,
        focused: Option<Uuid>,
    ) {
        displays.sort_by_key(|display| u64::from(display.id()));
        self.outputs = displays
            .into_iter()
            .map(|display| GhostShellOutput {
                is_primary: primary.is_some_and(|primary| {
                    display
                        .uuid()
                        .is_ok_and(|uuid| uuid == primary)
                }),
                is_focused: false,
                display,
            })
            .collect();
        if !self
            .outputs
            .iter()
            .any(|output| output.is_primary)
        {
            if let Some(output) = self.outputs.first_mut() {
                output.is_primary = true;
            }
        }
        self.set_focused_output(focused);
    }

    pub fn get_output(&self) -> Option<&GhostShellOutput> {
        self.get_focused_output()
            .or_else(|| self.get_primary_output())
    }

    pub fn get_outputs(&self) -> &Vec<GhostShellOutput> {
        &self.outputs
    }

    pub fn get_focused_output(&self) -> Option<&GhostShellOutput> {
        self.outputs
            .iter()
            .find(|display| display.is_focused)
    }

    pub fn get_primary_output(&self) -> Option<&GhostShellOutput> {
        self.outputs
            .iter()
            .find(|display| display.is_primary)
            .or_else(|| self.outputs.first())
    }

    pub fn get_displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        self.outputs
            .iter()
            .map(|output| output.display.clone())
            .collect::<Vec<_>>()
    }
}

impl Global for GhostShell {}

#[cfg(test)]
mod tests {
    use ghost_shell_gpui::{Bounds, DisplayId, Pixels};

    use super::*;

    #[derive(Debug)]
    struct Display(u64, &'static str);

    impl PlatformDisplay for Display {
        fn id(&self) -> DisplayId {
            DisplayId::new(self.0)
        }
        fn uuid(&self) -> anyhow::Result<Uuid> {
            Ok(Uuid::new_v5(&Uuid::NAMESPACE_DNS, self.1.as_bytes()))
        }
        fn bounds(&self) -> Bounds<Pixels> {
            Bounds::default()
        }
    }

    #[test]
    fn disconnected_primary_falls_back_and_reconnection_uses_new_handle() {
        let internal: Rc<dyn PlatformDisplay> = Rc::new(Display(1, "eDP-1"));
        let external: Rc<dyn PlatformDisplay> = Rc::new(Display(2, "DP-1"));
        let primary = external.uuid().ok();
        let mut shell = GhostShell::new(Vec::new());
        shell.set_displays(vec![internal.clone(), external], primary, primary);
        assert_eq!(shell.get_output().unwrap().display.id(), DisplayId::new(2));

        shell.set_displays(vec![internal.clone()], primary, primary);
        assert_eq!(shell.get_output().unwrap().display.id(), internal.id());
        assert!(shell.get_focused_output().is_none());
        assert!(
            shell
                .get_primary_output()
                .unwrap()
                .is_primary
        );

        let reconnected: Rc<dyn PlatformDisplay> = Rc::new(Display(3, "DP-1"));
        shell.set_displays(vec![internal, reconnected], primary, None);
        assert_eq!(shell.get_output().unwrap().display.id(), DisplayId::new(3));
    }

    #[test]
    fn no_connected_displays_is_valid() {
        let mut shell = GhostShell::new(Vec::new());
        shell.set_displays(vec![Rc::new(Display(1, "eDP-1"))], None, None);
        shell.set_displays(Vec::new(), None, None);
        assert!(shell.get_output().is_none());
        assert!(shell.get_primary_output().is_none());
        assert!(shell.get_displays().is_empty());
    }

    #[test]
    fn focus_can_be_cleared_without_retaining_an_old_output() {
        let internal: Rc<dyn PlatformDisplay> = Rc::new(Display(1, "eDP-1"));
        let external: Rc<dyn PlatformDisplay> = Rc::new(Display(2, "DP-1"));
        let focused = external.uuid().ok();
        let mut shell = GhostShell::new(Vec::new());
        shell.set_displays(vec![external, internal], None, focused);
        assert_eq!(shell.get_output().unwrap().display.id(), DisplayId::new(2));
        shell.set_focused_output(None);
        assert_eq!(shell.get_output().unwrap().display.id(), DisplayId::new(1));
    }
}
