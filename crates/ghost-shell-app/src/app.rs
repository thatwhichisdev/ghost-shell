use std::collections::HashMap;

use ghost_shell_bar::Bar;
use gpui::{Entity, Global, accesskit::Uuid};

use ghost_shell_launcher::Launcher;

pub struct GhostShell {
    pub launcher: Entity<Launcher>,
    pub bars: HashMap<Uuid, Entity<Bar>>,
}

impl GhostShell {
    #[must_use]
    pub fn new(
        launcher: Entity<Launcher>,
        bars: HashMap<Uuid, Entity<Bar>>,
    ) -> Self {
        Self { launcher, bars }
    }
}

impl Global for GhostShell {}
