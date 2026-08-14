use std::rc::Rc;

use gpui::{
    Context, Entity, IntoElement, ObjectFit, Pixels, Render, ScrollStrategy,
    Size, Subscription, Window, div, img, prelude::*, px, size,
};
use gpui_component::{
    ActiveTheme as _, Sizable, StyledExt, VirtualListScrollHandle,
    input::{Input, InputEvent, InputState, MoveDown, MoveUp},
    list::ListItem,
    v_virtual_list,
};
use neo_frizbee::Config;

use ghost_shell_actions::LauncherClose;
use ghost_shell_niri::NiriClient;

use crate::{
    actions::EntrySpawn,
    entries::{DesktopEntries, DesktopEntry},
};

/// Struct that represents launcher's view.
pub(crate) struct View {
    /// A strong well-typed reference to launcher's input
    input: Entity<InputState>,

    /// A handle to a input's subscription.
    /// When dropped, the subscription is cancelled and the callback will no longer be invoked.
    _input_subscription: Subscription,

    entries: Vec<DesktopEntry>,
    entries_sizes: Rc<Vec<Size<Pixels>>>,

    entries_filtered: Vec<usize>,
    entry_selected: Option<usize>,

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
        let _input_subscription =
            cx.subscribe_in(&input, window, Self::on_query_event);

        let entries = cx.global::<DesktopEntries>().clone();
        let entries_len = entries.items.len();
        let entries_filtered = (0..entries_len).collect();
        let entries_sizes = Rc::new(vec![size(px(0.0), px(48.0)); entries_len]);

        Self {
            input,
            _input_subscription,
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
                neo_frizbee::match_list(
                    query.trim(),
                    &self.entries,
                    &Config::default(),
                )
                .into_iter()
                .map(|matched| matched.index as usize)
                .collect()
            }
        };

        self.entry_selected = (!matched.is_empty()).then_some(0);
        self.entries_sizes =
            Rc::new(vec![size(px(0.0), px(48.0)); matched.len()]);
        self.entries_filtered = matched;

        if self.entry_selected.is_some() {
            self.scroll_handle.scroll_to_item(0, ScrollStrategy::Top);
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
        self.scroll_handle.scroll_to_item(ix, ScrollStrategy::Top);

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
        self.scroll_handle.scroll_to_item(ix, ScrollStrategy::Top);

        cx.notify();
    }

    fn on_entry_spawn(
        &mut self,
        _: &EntrySpawn,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(command) = self
            .entry_selected
            .map(|ix| self.entries.get(ix).unwrap())
            .map(|entry| entry.command.clone())
        else {
            return;
        };

        let action = ghost_shell_niri::Action::Spawn { command };
        let request = ghost_shell_niri::Request::Action(action);

        let reply = {
            let runtime = gpui_tokio::Tokio::handle(cx);
            let niri_client = cx.global_mut::<NiriClient>();

            runtime.block_on(niri_client.send(request))
        };

        match reply {
            Ok(Ok(ghost_shell_niri::Response::Handled)) => {
                cx.dispatch_action(&LauncherClose);
            }

            Ok(Ok(response)) => {
                eprintln!("Unexpected Niri response: {response:?}");
            }

            Ok(Err(error)) => {
                eprintln!("Niri failed to spawn application: {error}");
            }

            Err(error) => {
                eprintln!("Failed to communicate with Niri: {error:#}");
            }
        }
    }

    fn render_entries(
        &mut self,
        cx: &mut Context<'_, View>,
    ) -> impl IntoElement {
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
    ) -> Option<ListItem> {
        const ICON_SIZE: f32 = 40.0;

        let entry = self
            .entries_filtered
            .get(ix)
            .and_then(|&entry_ix| self.entries.get(entry_ix))?;

        let selected = self.entry_selected == Some(ix);

        let theme = cx.theme();

        let (title_color, description_color) = if selected {
            (theme.accent_foreground, theme.accent_foreground)
        } else {
            (theme.foreground, theme.muted_foreground)
        };

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
                    .text_size(px(14.0))
                    .font_bold()
                    .text_color(title_color)
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
                            .text_size(px(13.0))
                            .text_color(description_color)
                            .when(selected, |this| this.opacity(0.7))
                            .truncate()
                            .child(description),
                    )
                },
            );

        let entry = ListItem::new(("launcher-entry", ix))
            .selected(selected)
            .size_full()
            .rounded_md()
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap_x_3()
                    .overflow_hidden()
                    .child(icon)
                    .child(body),
            );

        Some(entry)
    }

    fn render_input(&mut self) -> impl IntoElement {
        div()
            .id("launcher-input")
            .w_full()
            .p_3()
            .border_b_1()
            .child(Input::new(&self.input).large().cleanable(true))
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
            .child(self.render_input())
            .child(self.render_entries(cx))
    }
}
