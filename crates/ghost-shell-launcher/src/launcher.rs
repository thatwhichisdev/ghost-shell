use std::ops::DerefMut;

use anyhow::{Context as _, Result};
use gpui::{
    AnyWindowHandle, App, Bounds, Context, Entity, Global, IntoElement, Render,
    Subscription, Svg, Task, Window, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions,
    accesskit::Uuid,
    div, img,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    prelude::*,
    px, rgb, size, svg,
};
use gpui_component::{
    ActiveTheme as _, IndexPath, Root, Sizable, StyledExt,
    input::{Input, InputEvent, InputState},
    list::{List, ListDelegate, ListItem, ListState},
};
use neo_frizbee::{Config, match_list_indices};

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
    input: Entity<InputState>,
    _input_subscription: Subscription,
    list: Entity<ListState<ApplicationListDelegate>>,
}

impl LauncherView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let apps = cx.global::<Applications>().clone();
        let list = cx.new(|cx| {
            ListState::new(
                ApplicationListDelegate {
                    all_items: apps.items.clone(),
                    filtered_items: apps.items,
                    index: None,
                },
                window,
                cx,
            )
        });

        let input = cx.new(|cx| InputState::new(window, cx));

        let _input_subscription =
            cx.subscribe_in(&input, window, Self::on_query_event);

        input.update(cx, |query, cx| {
            query.focus(window, cx);
        });

        Self {
            input,
            _input_subscription,
            list,
        }
    }

    fn on_query_event(
        &mut self,
        input: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, InputEvent::Change) {
            return;
        }

        let query = input.read(cx).value().to_string();

        self.list.update(cx, |list, _cx| {
            let query = query.trim();

            if query.is_empty() {
                let delegate_mut = list.delegate_mut();

                delegate_mut
                    .filtered_items
                    .clone_from(&delegate_mut.all_items);
            } else {
                let app_names = list
                    .delegate()
                    .all_items
                    .iter()
                    .map(|app| app.name.as_str())
                    .collect::<Vec<_>>();

                let matches =
                    match_list_indices(query, &app_names, &Config::default());

                list.delegate_mut().filtered_items = matches
                    .into_iter()
                    .filter_map(|matched| {
                        list.delegate()
                            .all_items
                            .get(matched.index as usize)
                            .cloned()
                    })
                    .collect();
            }
        });
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
                    .child(Input::new(&self.input).large().cleanable(true)),
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
    all_items: Vec<Application>,
    filtered_items: Vec<Application>,
    index: Option<IndexPath>,
}

impl ListDelegate for ApplicationListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.filtered_items.len()
    }

    fn render_item(
        &mut self,
        index: IndexPath,
        _window: &mut Window,
        cx: &mut Context<gpui_component::list::ListState<Self>>,
    ) -> Option<Self::Item> {
        const ICON_SIZE: f32 = 40.0;
        const ROW_HEIGHT: f32 = 48.0;

        let app = self.filtered_items.get(index.row)?;
        let selected = self.index == Some(index);

        let title_color = if selected {
            cx.theme().accent_foreground
        } else {
            cx.theme().foreground
        };

        let description_color = if selected {
            cx.theme().accent_foreground
        } else {
            cx.theme().muted_foreground
        };

        let description = app
            .desc
            .clone()
            .filter(|description| !description.trim().is_empty());

        let icon_column = div()
            .w(px(ICON_SIZE))
            .h(px(ICON_SIZE))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .when_some(app.icon.clone(), |this, icon_path| {
                this.child(
                    img(icon_path)
                        .size(px(ICON_SIZE))
                        .object_fit(gpui::ObjectFit::Contain),
                )
            });

        let text_column = div()
            .min_w_0()
            .flex_1()
            .flex()
            .flex_col()
            .justify_center()
            .overflow_hidden()
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .text_size(px(14.0))
                    .font_bold()
                    .text_color(title_color)
                    .truncate()
                    .child(app.name.clone()),
            )
            .when_some(description, |this, description| {
                this.child(
                    div()
                        .w_full()
                        .min_w_0()
                        .text_size(px(13.0))
                        .text_color(description_color)
                        .when(selected, |this| this.opacity(0.7))
                        .truncate()
                        .child(description),
                )
            });

        let content = div()
            .w_full()
            .min_w_0()
            .h_full()
            .flex()
            .items_center()
            .gap_x_3()
            .overflow_hidden()
            .child(icon_column)
            .child(text_column);

        Some(
            ListItem::new(index)
                .h(px(ROW_HEIGHT))
                // Override ListItem's default 12px horizontal padding.
                .px_2()
                .py_1()
                .rounded_md()
                .child(content)
                .selected(selected),
        )
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
