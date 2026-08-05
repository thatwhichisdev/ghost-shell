use anyhow::{Context as _, Result};
use gpui::{
    AnyWindowHandle, App, Bounds, Context, Entity, Global, IntoElement, Render,
    Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind,
    WindowOptions,
    accesskit::Uuid,
    div,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    prelude::*,
    px, rgb, size,
};
use gpui_component::{
    ActiveTheme as _, IndexPath, Root, Sizable, StyledExt,
    input::{Input, InputState},
    list::{List, ListDelegate, ListItem, ListState},
};

use crate::{Application, Applications};
use ghost_shell_niri::NiriState;

pub struct Launcher {
    handle: Option<AnyWindowHandle>,
}

impl Launcher {
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
            let view = cx.new(|cx| LauncherView::new(window, cx));
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

struct LauncherView {
    list: Entity<ListState<ApplicationListDelegate>>,
    query: Entity<InputState>,
}

impl LauncherView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let apps = cx.global::<Applications>().clone();
        let list = cx.new(|cx| {
            ListState::new(
                ApplicationListDelegate {
                    apps: apps.items,
                    index: None,
                },
                window,
                cx,
            )
        });

        let query = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search applications or files")
        });

        query.update(cx, |query, cx| {
            query.focus(window, cx);
        });

        Self { list, query }
    }
}

impl Render for LauncherView {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id("launcher")
            .key_context("launcher")
            .size_full()
            .rounded_lg()
            .flex()
            .flex_col()
            .bg(rgb(0x00_0000))
            .overflow_hidden()
            .text_color(cx.theme().colors.foreground)
            .child(
                div()
                    .id("launcher-input")
                    .w_full()
                    .p_3()
                    .border_b_1()
                    .child(Input::new(&self.query).large().cleanable(true)),
            )
            .child(
                div()
                    .id("launcher-results")
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(List::new(&self.list).size_full()),
            )
    }
}

impl Global for Launcher {}

struct ApplicationListDelegate {
    apps: Vec<Application>,
    index: Option<IndexPath>,
}

impl ListDelegate for ApplicationListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.apps.len()
    }

    fn render_item(
        &mut self,
        index: IndexPath,
        _window: &mut Window,
        _cx: &mut Context<gpui_component::list::ListState<Self>>,
    ) -> Option<Self::Item> {
        let app = self.apps.get(index.row)?;
        let content = div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .justify_start()
            .child(
                div()
                    .w_full()
                    .font_bold()
                    .truncate()
                    .child(app.name.clone()),
            )
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .text_sm()
                    .truncate()
                    .child(app.desc.clone().unwrap_or_default()),
            );

        let item = ListItem::new(index)
            .child(content)
            .selected(self.index == Some(index));

        Some(item)
    }

    fn set_selected_index(
        &mut self,
        index: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<gpui_component::list::ListState<Self>>,
    ) {
        self.index = index;
        cx.notify();
    }
}
