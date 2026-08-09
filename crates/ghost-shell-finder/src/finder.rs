use anyhow::{Context as _, Result};
use ghost_shell_niri::NiriState;
use gpui::{
    AnyWindowHandle, App, Bounds, Context, IntoElement, Render, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
    accesskit::Uuid,
    div,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    prelude::*,
    px, size,
};
use gpui_component::Root;

pub struct Finder {
    handle: Option<AnyWindowHandle>,
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
        let id = niri_state
            .clone()
            .workspaces
            .into_values()
            .find(|workspace| workspace.is_focused == true)
            .and_then(|workspace| workspace.output)
            .map(|output| Uuid::new_v5(&Uuid::NAMESPACE_DNS, output.as_bytes()))
            .unwrap(); // for now panic, but ideally we should toggle launcher on primary output if nothing is focused

        let display = cx
            .displays()
            .iter()
            .find(|display| display.uuid().is_ok_and(|uuid| uuid == id))
            .cloned()
            .unwrap(); // for now display should be always present in the config, will change later

        let bounds = Bounds::centered(
            Some(display.id()),
            size(px(540.0), px(450.0)),
            cx,
        );

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            focus: true,
            show: true,
            kind: WindowKind::LayerShell(LayerShellOptions {
                namespace: "ghost-shell-launcher".to_owned(),
                layer: Layer::Overlay,
                anchor: Anchor::empty(),
                exclusive_zone: None,
                exclusive_edge: None,
                margin: None,
                keyboard_interactivity: KeyboardInteractivity::OnDemand,
            }),
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            display_id: Some(display.id()),
            window_background: WindowBackgroundAppearance::Transparent,
            app_id: Some("ghost-shell-launcher".to_owned()),
            ..Default::default()
        };

        let handle = cx.open_window(window_options, |window, cx| {
            let view = cx.new(|_cx| View {});
            cx.new(|cx| Root::new(view, window, cx).bordered(false))
        })?;

        self.handle = Some(handle.into());

        Ok(())
    }

    pub fn close(&mut self, cx: &mut App) -> Result<()> {
        let Some(handle) = self.handle.take() else {
            return Ok(());
        };

        handle
            .update(cx, |_view, window, _cx| {
                window.remove_window();
            })
            .context("failed to close launcher window")?;

        self.handle = None;

        Ok(())
    }
}

struct View;

impl Render for View {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().id("finder").key_context("finder")
    }
}
