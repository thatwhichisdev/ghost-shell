use std::collections::HashMap;

use ghost_shell_bar::Bar;
use gpui::{Entity, Global, accesskit::Uuid};

pub struct GhostShell {
    pub bars: HashMap<Uuid, Entity<Bar>>,
}

impl GhostShell {
    #[must_use]
    pub fn new(bars: HashMap<Uuid, Entity<Bar>>) -> Self {
        Self { bars }
    }
}

impl Global for GhostShell {}
