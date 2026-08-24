use std::{rc::Rc, sync::Arc, time::Duration};

use gpui::{
    Context, Div, Entity, IntoElement, Pixels, Render, ScrollStrategy, Size, Stateful,
    Window, div, prelude::*, px, size,
};
use gpui_component::{
    ActiveTheme as _, IconName, Sizable, VirtualListScrollHandle,
    input::{Input, InputEvent, InputState, MoveDown, MoveUp},
    spinner::Spinner,
    v_virtual_list,
};

use crate::search::{Search, SearchItem, SearchOptions};

pub(crate) struct View {
    search: Arc<Search>,

    query: Entity<InputState>,

    entries: Vec<SearchItem>,
    entries_sizes: Rc<Vec<Size<Pixels>>>,

    entry_selected: Option<usize>,
    entry_hovered: Option<usize>,

    scroll_handle: VirtualListScrollHandle,

    is_indexing: bool,
    is_searching: bool,

    indexed_entries: usize,
    matched_entries: usize,
}

impl View {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = Arc::new(Search::try_new(SearchOptions::default()).unwrap());
        let query = cx.new(|cx| {
            let query = InputState::new(window, cx);
            query.focus(window, cx);
            query
        });

        cx.subscribe_in(&query, window, Self::on_input_query_event)
            .detach();

        Self::spawn_scan(window, cx);

        Self {
            search,
            query,
            entries: vec![],
            entries_sizes: Rc::new(vec![]),
            entry_selected: None,
            entry_hovered: None,
            scroll_handle: VirtualListScrollHandle::new(),
            is_indexing: true,
            is_searching: false,
            indexed_entries: 0,
            matched_entries: 0,
        }
    }

    fn on_input_query_event(
        &mut self,
        query: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change | InputEvent::Focus | InputEvent::Blur => {}
            InputEvent::PressEnter {
                secondary: _,
                shift: _,
            } => {
                let needle = query.read(cx).value().trim().to_owned();
                self.spawn_search(needle, window, cx);
            }
        }
    }

    fn on_item_select_next(
        &mut self,
        _: &MoveDown,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let len = self.entries.len();

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

    fn on_item_select_previous(
        &mut self,
        _: &MoveUp,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let len = self.entries.len();

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

    fn spawn_scan(window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;

                let is_indexing = view
                    .update(cx, |view, cx| {
                        let scan_progress = view.search.get_scan_progress().unwrap();
                        let is_indexing = scan_progress.is_scanning;
                        let entries_count = scan_progress.scanned_files_count;

                        view.is_indexing = is_indexing;
                        view.indexed_entries = entries_count;

                        cx.notify();

                        is_indexing
                    })
                    .unwrap();

                if !is_indexing {
                    break;
                }
            }
        })
        .detach();
    }

    fn spawn_search(
        &mut self,
        needle: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_searching = true;
        cx.notify();

        let search = self.search.clone();
        let search_task = cx
            .background_executor()
            .spawn(async move { search.search(&needle, 100) });

        cx.spawn_in(window, async move |this, cx| {
            let Ok(search_result) = search_task.await else {
                this.update(cx, |view, cx| {
                    view.is_searching = false;
                    cx.notify();
                })?;

                anyhow::bail!("failed to search")
            };

            this.update(cx, |view, cx| {
                view.entry_selected = if search_result.items.is_empty() {
                    None
                } else {
                    Some(0)
                };

                view.indexed_entries = search_result.indexed_files;
                view.matched_entries = search_result.matched;
                view.entries = search_result.items;
                view.entries_sizes = Rc::new(vec![
                    size(
                        px(0.0),
                        gpui_component::Size::XSmall.table_row_height()
                    );
                    view.entries.len()
                ]);

                view.is_searching = false;

                cx.notify();
            })?;

            anyhow::Ok(())
        })
        .detach();
    }

    fn render_input(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("finder-input-query")
            .w_full()
            .h_9()
            .px_2p5()
            .flex_none()
            .flex()
            .items_center()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                Input::new(&self.query)
                    .appearance(false)
                    .text_base()
                    .disabled(self.is_indexing),
            )
    }

    fn render_loading(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("finder-loading")
            .flex_1()
            .min_h_0()
            .flex()
            .items_center()
            .justify_center()
            .child(
                Spinner::new()
                    .icon(IconName::LoaderCircle)
                    .large()
                    .color(cx.theme().foreground),
            )
    }

    fn render_entry(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let item = self.entries.get(index)?;
        let is_selected = self.entry_selected == Some(index);
        let is_hovered = self.entry_hovered == Some(index);

        let background = if is_selected {
            cx.theme().list_active
        } else if is_hovered {
            cx.theme().list_hover
        } else {
            cx.theme().background
        };

        Some(
            div()
                .id(("finder-result", index))
                .w_full()
                .h_full()
                .px(px(4.0))
                .py(px(2.0))
                .on_hover(cx.listener(move |view, is_hovered: &bool, _, cx| {
                    view.entry_hovered = is_hovered.then_some(index);
                    cx.notify();
                }))
                .child(
                    div()
                        .w_full()
                        .h_full()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .px_1p5()
                        .rounded_sm()
                        .bg(background)
                        .child(
                            div()
                                .min_w_0()
                                .w_full()
                                .truncate()
                                .text_base()
                                .child(item.path.to_string_lossy().to_string()),
                        ),
                ),
        )
    }

    fn render_entries(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("finder-list")
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(
                v_virtual_list(
                    cx.entity().clone(),
                    "finder-virtual-list",
                    self.entries_sizes.clone(),
                    |view, visible_range, _, cx| {
                        visible_range
                            .filter_map(|index| view.render_entry(index, cx))
                            .collect()
                    },
                )
                .track_scroll(&self.scroll_handle),
            )
    }

    fn render_status(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let status = if self.is_indexing {
            "Indexing...".to_owned()
        } else if self.is_searching {
            "Searching...".to_owned()
        } else {
            format!("{} matches", self.matched_entries)
        };

        let files = if self.is_indexing {
            format!("{} files indexed", self.indexed_entries)
        } else {
            format!("{} files", self.indexed_entries)
        };

        div()
            .id("finder-status")
            .w_full()
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .p_1p5()
            .border_t_1()
            .border_color(cx.theme().border)
            .text_base()
            .text_color(cx.theme().foreground)
            .child(status)
            .child(files)
    }
}

impl Render for View {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_loading = self.is_indexing || self.is_searching;

        div()
            .id("finder")
            .key_context("finder")
            .on_action(cx.listener(Self::on_item_select_previous))
            .on_action(cx.listener(Self::on_item_select_next))
            .overflow_hidden()
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().colors.background)
            .text_color(cx.theme().colors.foreground)
            .child(self.render_input(cx))
            .when(is_loading, |this| this.child(self.render_loading(cx)))
            .when(!is_loading, |this| this.child(self.render_entries(cx)))
            .child(self.render_status(cx))
    }
}
