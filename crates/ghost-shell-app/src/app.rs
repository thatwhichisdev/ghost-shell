use std::collections::HashMap;

use ghost_shell_bar::Bar;
use gpui::{DisplayId, Entity, Global};

use ghost_shell_launcher::Launcher;

pub struct GhostShell {
    launcher: Entity<Launcher>,
    bars: HashMap<DisplayId, Entity<Bar>>,
}

impl GhostShell {
    #[must_use]
    pub fn new(
        launcher: Entity<Launcher>,
        bars: HashMap<DisplayId, Entity<Bar>>,
    ) -> Self {
        Self { launcher, bars }
    }
}

impl Global for GhostShell {}
