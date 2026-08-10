use anyhow::{Context as _, Result};
use ghost_shell_app::GhostShell;
use gpui::{
    App, AppContext, Bounds, Global, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    px, size,
};
use gpui_component::Root;

use crate::view::View;

pub struct Finder {
    handle: Option<WindowHandle<Root>>,
}

impl Finder {
    #[must_use]
    pub fn new() -> Self {
        Self { handle: None }
    }

    pub fn toggle(&mut self, cx: &mut App) -> Result<()> {
        if self.handle.is_some() {
            self.close(cx)
        } else {
            self.open(cx)
        }
    }

    pub fn open(&mut self, cx: &mut App) -> Result<()> {
        let ghost_shell = cx.global::<GhostShell>();
        let output = ghost_shell
            .get_focused_output()
            .unwrap_or(ghost_shell.get_primary_output());

        let window_bounds = WindowBounds::Windowed(Bounds::centered(
            Some(output.display.id()),
            size(px(700.0), px(500.0)),
            cx,
        ));

        let window_kind = WindowKind::LayerShell(LayerShellOptions {
            namespace: "ghost-shell-finder".to_owned(),
            layer: Layer::Overlay,
            anchor: Anchor::empty(),
            exclusive_zone: None,
            exclusive_edge: None,
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
        });

        let window_options = WindowOptions {
            window_bounds: Some(window_bounds),
            titlebar: None,
            kind: window_kind,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            display_id: Some(output.display.id()),
            window_background: WindowBackgroundAppearance::Blurred,
            app_id: Some("ghost-shell-finder".to_owned()),
            ..Default::default()
        };

        let handle = cx.open_window(window_options, |window, cx| {
            let view = cx.new(|cx| View::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx).bordered(false))
        })?;

        self.handle = Some(handle);

        Ok(())
    }

    pub fn close(&mut self, cx: &mut App) -> Result<()> {
        match self.handle.take() {
            Some(handle) => {
                handle
                    .update(cx, |_view, window, _cx| window.remove_window())
                    .context("failed to close launcher window")?;

                self.handle = None;

                Ok(())
            }
            None => Ok(()),
        }
    }
}

impl Global for Finder {}
