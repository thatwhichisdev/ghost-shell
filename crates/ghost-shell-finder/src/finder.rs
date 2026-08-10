use anyhow::{Context as _, Result};
use gpui::{
    App, AppContext, Bounds, Global, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions,
    accesskit::Uuid,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    px, size,
};
use gpui_component::Root;

use ghost_shell_niri::NiriState;

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
        let niri_state = cx.global::<NiriState>();

        // get niri display id of the focused display
        let id = niri_state
            .clone()
            .workspaces
            .into_values()
            .find(|workspace| workspace.is_focused == true)
            .and_then(|workspace| workspace.output)
            .map(|output| Uuid::new_v5(&Uuid::NAMESPACE_DNS, output.as_bytes()))
            .unwrap(); // for now panic, but ideally we should toggle launcher on primary output if nothing is focused

        // convert niri id into gpui id
        let display = cx
            .displays()
            .iter()
            .find(|display| display.uuid().is_ok_and(|uuid| uuid == id))
            .cloned()
            .unwrap(); // for now display should be always present in the config, will change later

        let window_bounds = WindowBounds::Windowed(Bounds::centered(
            Some(display.id()),
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
            display_id: Some(display.id()),
            window_background: WindowBackgroundAppearance::Transparent,
            app_id: Some("ghost-shell-finder".to_owned()),
            ..Default::default()
        };

        let handle = cx.open_window(window_options, |window, cx| {
            let view = cx.new(|_cx| View {});
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
