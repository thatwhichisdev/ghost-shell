use gpui::{
    App, Context, Entity, IntoElement, Render, Subscription, Window, div, img,
    prelude::*, px,
};
use gpui_component::{
    ActiveTheme as _, IndexPath, Sizable, StyledExt,
    input::{Input, InputEvent, InputState, MoveDown, MoveUp},
    list::{List, ListDelegate, ListItem, ListState},
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
    #[allow(unused)]
    input_subscription: Subscription,

    /// A strong well-typed reference to launcher's list of applications and files
    list: Entity<ListState<ApplicationListDelegate>>,
}

impl View {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let entries = cx.global::<DesktopEntries>().clone();

        // Initialize list of applications and select first item
        let list = cx.new(|cx| {
            let all_indices = (0..entries.items.len()).collect();
            let delegate = ApplicationListDelegate {
                entries: entries.items,
                entries_filtered: all_indices,
                entry_selected: None,
            };
            let mut list = ListState::new(delegate, window, cx);

            list.set_selected_index(Some(IndexPath::new(0)), window, cx);
            list
        });

        // Initialiaze input and focus it
        let input = cx.new(|cx| {
            let input = InputState::new(window, cx);

            input.focus(window, cx);
            input
        });

        // Subscirbe to input's events and keep the handle
        let input_subscription =
            cx.subscribe_in(&input, window, Self::on_query_event);

        Self {
            input,
            input_subscription,
            list,
        }
    }

    fn on_query_event(
        &mut self,
        input: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, InputEvent::Change) {
            return;
        }

        let query = input.read(cx).value();
        let query = query.trim();

        self.list.update(cx, |list, cx| {
            let has_items = {
                let delegate = list.delegate_mut();
                delegate.filter(query);
                !delegate.entries_filtered.is_empty()
            };

            let selected = has_items.then_some(IndexPath::new(0));

            list.set_selected_index(selected, window, cx);
            cx.notify();
        });
    }

    fn on_entry_select_next(
        &mut self,
        _: &MoveDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.list.update(cx, |list, cx| {
            let items_count = list.delegate().entries_filtered.len();
            let next_index = match (items_count, list.selected_index()) {
                (0, _) => None,
                (_, None) => Some(IndexPath::new(0)),
                (items_count, Some(current)) => {
                    let next_row = if current.row + 1 >= items_count {
                        0
                    } else {
                        current.row + 1
                    };

                    Some(IndexPath::new(next_row))
                }
            };

            if next_index.is_some() {
                list.set_selected_index(next_index, window, cx);
                list.scroll_to_selected_item(window, cx);
            }
        });
    }

    fn on_entry_select_previous(
        &mut self,
        _: &MoveUp,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.list.update(cx, |list, cx| {
            let items_count = list.delegate().entries_filtered.len();
            let previous_index = match (items_count, list.selected_index()) {
                (0, _) => None,
                (items_count, None) => Some(IndexPath::new(items_count - 1)),
                (items_count, Some(current)) => {
                    let previous_row =
                        if current.row == 0 || current.row >= items_count {
                            items_count - 1
                        } else {
                            current.row - 1
                        };

                    Some(IndexPath::new(previous_row))
                }
            };

            if previous_index.is_some() {
                list.set_selected_index(previous_index, window, cx);
                list.scroll_to_selected_item(window, cx);
            }
        });
    }

    fn on_entry_spawn(
        &mut self,
        _: &EntrySpawn,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(command) = self
            .list
            .read(cx)
            .delegate()
            .selected_item()
            .map(|application| application.command.clone())
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
                    .id("launcher-entries")
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(List::new(&self.list).size_full()),
            )
    }
}

struct ApplicationListDelegate {
    entries: Vec<DesktopEntry>,
    entries_filtered: Vec<usize>,
    entry_selected: Option<IndexPath>,
}

impl ApplicationListDelegate {
    fn item(&self, row: usize) -> Option<&DesktopEntry> {
        let item_index = *self.entries_filtered.get(row)?;
        self.entries.get(item_index)
    }

    fn selected_item(&self) -> Option<&DesktopEntry> {
        let selected = self.entry_selected?;
        self.item(selected.row)
    }

    fn filter(&mut self, query: &str) {
        self.entries_filtered.clear();

        if query.is_empty() {
            self.entries_filtered.extend(0..self.entries.len());
            return;
        }

        let matches =
            neo_frizbee::match_list(query, &self.entries, &Config::default());

        self.entries_filtered.reserve(matches.len());
        self.entries_filtered
            .extend(matches.into_iter().map(|matched| matched.index as usize));
    }
}

impl ListDelegate for ApplicationListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.entries_filtered.len()
    }

    fn render_item(
        &mut self,
        index: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        const ICON_SIZE: f32 = 40.0;
        const ROW_HEIGHT: f32 = 48.0;

        let app = self.item(index.row)?;
        let selected = self.entry_selected == Some(index);

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
            .description
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
        cx: &mut Context<ListState<Self>>,
    ) {
        self.entry_selected = index;
        cx.notify();
    }
}
