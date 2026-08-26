use std::{rc::Rc, sync::Arc, time::Duration};

use gpui::{
    Context, Div, Entity, IntoElement, Pixels, Render, ScrollStrategy, Size, Stateful,
    Task, Window, div, prelude::*, px, size,
};
use gpui_component::{
    ActiveTheme as _, IconName, Sizable, VirtualListScrollHandle,
    input::{Input, InputEvent, InputState, MoveDown, MoveUp},
    spinner::Spinner,
    v_virtual_list,
};

use crate::{
    FinderOpenSelected,
    search::{Search, SearchItem, SearchOptions},
};

/// Struct that represtents state of finder's UI
pub(crate) struct FinderView {
    /// Thread-safe pointer to the Search service
    search: Arc<Search>,
    /// Identifies the newest requested search.
    search_generation: u64,
    /// Keeps the current search continuation alive and cancels it when replaced.
    search_task: Option<Task<()>>,
    /// Strong reference to finder's input
    query: Entity<InputState>,
    /// Vector that represents search result entries
    entries: Vec<SearchItem>,
    /// Vector that represents sizes of the search result entries
    entries_sizes: Rc<Vec<Size<Pixels>>>,
    /// Index of selected entry if exists, otherwise empty
    entry_selected: Option<usize>,
    /// Index of mouse hovered entry if exists, otherwise empty
    entry_hovered: Option<usize>,
    /// Scroll handle for the virtual list
    scroll_handle: VirtualListScrollHandle,
    /// Identifies if indexing task is running
    is_indexing: bool,
    /// Identifies if search task is running
    is_searching: bool,
    /// Number of all indexed entries on the given base path
    indexed_entries: usize,
    /// Number of entries that matched query
    matched_entries: usize,
}

impl FinderView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = Arc::new(Search::try_new(SearchOptions::default()).unwrap());
        let query = cx.new(|cx| {
            let query = InputState::new(window, cx);
            query.focus(window, cx);
            query
        });

        cx.subscribe_in(&query, window, Self::on_input_query_event)
            .detach();

        Self::spawn_indexing(cx).detach();

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
            search_generation: 0,
            search_task: None,
            indexed_entries: 0,
            matched_entries: 0,
        }
    }

    fn on_input_query_event(
        &mut self,
        query: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change | InputEvent::Focus | InputEvent::Blur => {}
            InputEvent::PressEnter {
                secondary: _,
                shift: _,
            } => {
                let needle = query.read(cx).value().trim().to_owned();
                self.spawn_search(needle, cx);
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

    fn on_item_open(
        &mut self,
        _: &FinderOpenSelected,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        let Some(entry) = self
            .entry_selected
            .and_then(|index| self.entries.get(index))
        else {
            return;
        };

        // todo: implement spawning logic via niri client, that will allow to open a file location using `xdg-open` command

        log::info!("Selected finder entry: {entry:#?}");
    }

    fn on_item_preview(
        &mut self,
        _: &FinderOpenSelected,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        let Some(entry) = self
            .entry_selected
            .and_then(|index| self.entries.get(index))
        else {
            return;
        };

        // todo: implement preview logic, if file is text based then we can preview it in additional window, take inspirantion from zed's file picker

        log::info!("Selected finder entry: {entry:#?}");
    }

    fn spawn_indexing(cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;

                let keep_polling = match view.update(cx, |view, cx| {
                    match view.search.get_scan_progress() {
                        Ok(progress) => {
                            view.is_indexing = progress.is_scanning;
                            view.indexed_entries = progress.scanned_files_count;
                            cx.notify();

                            progress.is_scanning
                        }
                        Err(error) => {
                            log::error!(
                                "Failed to read file indexing progress: {error:#}"
                            );

                            view.is_indexing = false;
                            cx.notify();

                            false
                        }
                    }
                }) {
                    Ok(keep_polling) => keep_polling,
                    Err(_) => {
                        log::warn!(
                            "Finder's view is dropped, cancelling indexing progress task"
                        );
                        break;
                    }
                };

                if !keep_polling {
                    break;
                }
            }
        })
    }

    fn spawn_search(&mut self, needle: String, cx: &mut Context<Self>) {
        self.is_searching = true;
        self.search_task.take();
        self.search_generation = self.search_generation.wrapping_add(1);

        cx.notify();

        if needle.is_empty() {
            self.is_searching = false;
            self.entry_selected = None;
            self.entry_hovered = None;
            self.entries.clear();
            self.entries_sizes = Rc::new(Vec::new());
            self.matched_entries = 0;
            cx.notify();
            return;
        }

        let search_generation = self.search_generation;
        let search = self.search.clone();
        let search_task = cx
            .background_executor()
            .spawn(async move { search.search(&needle, 100) });

        self.search_task = Some(cx.spawn(async move |this, cx| {
            let search_result = search_task.await;
            let _ = this.update(cx, |view, cx| {
                // A newer request was started while this search was running.
                if view.search_generation != search_generation {
                    return;
                }

                view.is_searching = false;

                match search_result {
                    Ok(result) => {
                        view.entries = result.items;
                        view.entries_sizes = Rc::new(vec![
                            size(
                                px(0.0),
                                gpui_component::Size::XSmall.table_row_height(),
                            );
                            view.entries.len()
                        ]);
                        view.entry_selected = (!view.entries.is_empty()).then_some(0);
                        view.entry_hovered = None;

                        view.indexed_entries = result.indexed_files;
                        view.matched_entries = result.matched;
                    }

                    Err(error) => {
                        log::error!("Failed to search files: {error:#}");
                    }
                }

                cx.notify();
            });
        }));
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

impl Render for FinderView {
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
            .on_action(cx.listener(Self::on_item_open))
            .on_action(cx.listener(Self::on_item_preview))
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
