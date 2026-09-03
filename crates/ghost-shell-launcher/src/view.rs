use std::rc::Rc;

use ghost_shell_actions::LauncherClose;
use ghost_shell_niri::NiriClient;
use gpui::{
    App, Context, Div, Entity, IntoElement, ObjectFit, Pixels, Render, ScrollStrategy,
    Size, Stateful, Subscription, Window, div, img, prelude::*, px, size,
};
use gpui_component::{
    ActiveTheme as _, StyledExt, VirtualListScrollHandle,
    input::{Input, InputEvent, InputState, MoveDown, MoveUp},
    v_virtual_list,
};
use neo_frizbee::Config;

use crate::{
    actions::EntrySpawn,
    entries::{DesktopEntries, DesktopEntry},
};

const INPUT_HEIGHT: f32 = 36.0;
const INPUT_PADDING_X: f32 = 4.0;

const ENTRY_GAP: f32 = 6.0;
const ICON_SIZE: f32 = 32.0;

const TEXT_SIZE: f32 = 14.0;
const TEXT_SIZE_SMALL: f32 = 12.0;

const FOOTER_PADDING: f32 = 6.0;

/// Struct that represents launcher's view.
pub(crate) struct View {
    /// A strong well-typed reference to launcher's input
    input: Entity<InputState>,
    /// A handle to a input's subscription.
    /// When dropped, the subscription is cancelled and the callback will no longer be invoked.
    #[allow(unused)]
    input_subscription: Subscription,
    /// Vector that represents desktop entries available within the system
    entries: Vec<DesktopEntry>,
    /// Vector that represents entry sizes within the virtual list
    entries_sizes: Rc<Vec<Size<Pixels>>>,
    /// Vector that stores indices of query matched entries while performing search
    entries_filtered: Vec<usize>,
    /// Index of selected entry if exists, otherwise empty
    entry_selected: Option<usize>,
    /// Scroll handle for the virtual list
    scroll_handle: VirtualListScrollHandle,
}

impl View {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Initialiaze input and focus it
        let input = cx.new(|cx| {
            let input = InputState::new(window, cx);
            input.focus(window, cx);
            input
        });

        // Subscirbe to input's events and keep the handle
        let input_subscription = cx.subscribe_in(&input, window, Self::on_query_event);

        let entries = cx.global::<DesktopEntries>().clone();
        let entries_len = entries.items.len();
        let entries_filtered = (0..entries_len).collect();
        let entries_sizes = Rc::new(vec![size(px(0.0), px(48.0)); entries_len]);

        Self {
            input,
            input_subscription,
            entries: entries.items,
            entries_sizes,
            entries_filtered,
            entry_selected: (entries_len > 0).then_some(0),
            scroll_handle: VirtualListScrollHandle::new(),
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

        let query = input.read(cx).value();

        let matched: Vec<usize> = {
            if query.is_empty() {
                (0..self.entries.len()).collect()
            } else {
                neo_frizbee::match_list(query.trim(), &self.entries, &Config::default())
                    .into_iter()
                    .map(|matched| matched.index as usize)
                    .collect()
            }
        };

        self.entry_selected = (!matched.is_empty()).then_some(0);
        self.entries_sizes = Rc::new(vec![size(px(0.0), px(48.0)); matched.len()]);
        self.entries_filtered = matched;

        if self.entry_selected.is_some() {
            self.scroll_handle
                .scroll_to_item(0, ScrollStrategy::Top);
        }

        cx.notify();
    }

    fn on_entry_select_next(
        &mut self,
        _: &MoveDown,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let len = self.entries_filtered.len();

        let Some(ix) = (len > 0).then(|| {
            self.entry_selected
                .filter(|&ix| ix < len)
                .map_or(0, |ix| (ix + 1) % len)
        }) else {
            self.entry_selected = None;
            return;
        };

        self.entry_selected = Some(ix);
        self.scroll_handle
            .scroll_to_item(ix, ScrollStrategy::Top);

        cx.notify();
    }

    fn on_entry_select_previous(
        &mut self,
        _: &MoveUp,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let len = self.entries_filtered.len();

        let Some(ix) = (len > 0).then(|| {
            self.entry_selected
                .filter(|&ix| ix < len)
                .and_then(|ix| ix.checked_sub(1))
                .unwrap_or(len - 1)
        }) else {
            self.entry_selected = None;
            return;
        };

        self.entry_selected = Some(ix);
        self.scroll_handle
            .scroll_to_item(ix, ScrollStrategy::Top);

        cx.notify();
    }

    // todo: move spawning logic into separate file?
    fn on_entry_spawn(
        &mut self,
        _: &EntrySpawn,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self
            .entry_selected
            .and_then(|ix| self.entries_filtered.get(ix))
            .and_then(|&ix| self.entries.get(ix))
            .filter(|entry| !entry.command.is_empty())
        else {
            return;
        };

        // todo: get terminal from the environment variable
        let command = if entry.terminal {
            ["ghostty", "-e"]
                .into_iter()
                .map(String::from)
                .chain(entry.command.iter().cloned())
                .collect()
        } else {
            entry.command.clone()
        };

        let spawn_task = gpui_tokio::Tokio::spawn_result(cx, async move {
            let mut niri_client = NiriClient::try_new().await?;
            niri_client.spawn(command).await
        });

        cx.spawn_in(window, async move |view, cx| match spawn_task.await {
            Ok(()) => {
                if let Err(err) = view.update_in(cx, |_view, window, cx| {
                    window.dispatch_action(Box::new(LauncherClose), cx);
                }) {
                    eprintln!("Failed to dispatch launcher close: {err:#}");
                }
            }
            Err(err) => {
                eprintln!("Failed to spawn application: {err:#}");
            }
        })
        .detach();
    }

    fn render_entries(&mut self, cx: &mut Context<'_, View>) -> impl IntoElement {
        div()
            .id("launcher-entries")
            .w_full()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(
                v_virtual_list(
                    cx.entity().clone(),
                    "launcher-virtual-entries",
                    self.entries_sizes.clone(),
                    |view, visible_range, _window, cx| {
                        visible_range
                            .filter_map(|ix| view.render_entry(ix, cx))
                            .collect()
                    },
                )
                .track_scroll(&self.scroll_handle),
            )
    }

    fn render_entry(
        &mut self,
        ix: usize,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let entry = self
            .entries_filtered
            .get(ix)
            .and_then(|&entry_ix| self.entries.get(entry_ix))?;

        let selected = self.entry_selected == Some(ix);
        let theme = cx.theme();

        let icon = div()
            .size(px(ICON_SIZE))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .when_some(entry.icon.clone(), |this, path| {
                this.child(
                    img(path)
                        .size(px(ICON_SIZE))
                        .object_fit(ObjectFit::Contain),
                )
            });

        let body = div()
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
                    .text_size(px(TEXT_SIZE))
                    .font_semibold()
                    .truncate()
                    .child(entry.name.clone()),
            )
            .when_some(
                entry
                    .description
                    .clone()
                    .filter(|description| !description.trim().is_empty()),
                |this, description| {
                    this.child(
                        div()
                            .w_full()
                            .min_w_0()
                            .text_size(px(TEXT_SIZE_SMALL))
                            .text_color(theme.foreground)
                            .truncate()
                            .child(description),
                    )
                },
            );

        Some(
            div()
                .id(("launcher-entry", ix))
                .w_full()
                .h_full()
                .px(px(4.0))
                .py(px(2.0))
                .child(
                    // Actual visual / selection surface.
                    div()
                        .w_full()
                        .h_full()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(ENTRY_GAP))
                        .px(px(6.0))
                        .rounded_sm()
                        .when(selected, |this| this.bg(cx.theme().list_active))
                        .hover(|style| style.bg(cx.theme().list_hover))
                        .child(icon)
                        .child(body),
                ),
        )
    }

    fn render_input(&mut self, cx: &mut App) -> impl IntoElement {
        div()
            .id("launcher-input")
            .w_full()
            .h(px(INPUT_HEIGHT))
            .px(px(INPUT_PADDING_X))
            .flex_none()
            .flex()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                Input::new(&self.input)
                    .appearance(false)
                    .text_size(px(TEXT_SIZE)),
            )
    }

    fn render_status(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let total = self.entries.len();
        let filtered = self.entries_filtered.len();

        div()
            .id("launcher-status")
            .w_full()
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .p(px(FOOTER_PADDING))
            .border_t_1()
            .border_color(cx.theme().border)
            .text_size(px(TEXT_SIZE_SMALL))
            .text_color(cx.theme().foreground)
            .child(format!("{filtered} matches"))
            .child(format!("{total} applications"))
    }
}

impl Render for View {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id("launcher")
            .key_context("launcher")
            .on_action(cx.listener(Self::on_entry_select_previous))
            .on_action(cx.listener(Self::on_entry_select_next))
            .on_action(cx.listener(Self::on_entry_spawn))
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(self.render_input(cx))
            .child(self.render_entries(cx))
            .child(self.render_status(cx))
    }
}
