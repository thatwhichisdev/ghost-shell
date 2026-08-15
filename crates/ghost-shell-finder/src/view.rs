use std::{rc::Rc, time::Duration};

use gpui::{
    Context, Entity, IntoElement, Pixels, Render, SharedString, Size, Window,
    div, prelude::*, px, size,
};
use gpui_component::{
    ActiveTheme as _, IconName, Sizable, VirtualListScrollHandle,
    form::field,
    input::{Input, InputEvent, InputState},
    spinner::Spinner,
    v_virtual_list,
};

use crate::search::{Search, SearchItem, SearchOptions};

pub(crate) struct View {
    search: Search,

    input_query: Entity<InputState>,

    items: Vec<SearchItem>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    item_selected: Option<usize>,
    scroll_handle: VirtualListScrollHandle,

    search_result: SharedString,

    is_scanning: bool,
}

impl View {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = Search::try_new(SearchOptions::default()).unwrap();
        let input_query = cx.new(|cx| InputState::new(window, cx));

        cx.subscribe_in(&input_query, window, Self::on_input_query_event)
            .detach();

        let executor = cx.background_executor().clone();

        cx.spawn_in(window, async move |view, cx| {
            loop {
                executor.timer(Duration::from_millis(100)).await;

                let is_scanning = view
                    .update(cx, |view, cx| {
                        let scan_progress =
                            view.search.get_scan_progress().unwrap();
                        let is_scanning = scan_progress.is_scanning;
                        let count = scan_progress.scanned_files_count;

                        view.is_scanning = is_scanning;
                        view.search_result =
                            format!("{count} files indexed").into();

                        cx.notify();

                        is_scanning
                    })
                    .unwrap();

                if !is_scanning {
                    break;
                }
            }
        })
        .detach();

        Self {
            search,
            input_query,
            items: vec![],
            item_sizes: Rc::new(vec![]),
            item_selected: None,
            scroll_handle: VirtualListScrollHandle::new(),
            search_result: SharedString::default(),
            is_scanning: true,
        }
    }

    fn on_input_query_event(
        &mut self,
        input: &Entity<InputState>,
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
                let needle = input.read(cx).value().trim().to_owned();
                self.search(needle, window, cx);
            }
        }
    }

    fn search(
        &mut self,
        needle: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.spawn_in(window, async move |this, cx| {
            this.update(cx, |view, _cx| {
                let search_result = view.search.search(&needle, 1000).unwrap();
                let indexed_files = search_result.indexed_files;
                let indexed_dirs = search_result.indexed_dirs;
                let matched = search_result.matched;

                view.search_result = format!(
                    "{indexed_files} files indexed • {indexed_dirs} dirs indexed • {matched} matched"
                )
                .into();

                view.item_selected = if search_result.items.is_empty() {
                    None
                } else {
                    Some(0)
                };

                view.items = search_result.items;
                view.item_sizes =
                    Rc::new(vec![size(px(900.0), px(19.0)); view.items.len()]);
            })
            .unwrap();
        })
        .detach();
    }
}

impl Render for View {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id("finder")
            .key_context("finder")
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().colors.background)
            .text_color(cx.theme().colors.foreground)
            .overflow_hidden()
            .child(
                div().id("finder-input-query").p_3().border_b_1().child(
                    field().child(
                        Input::new(&self.input_query)
                            .disabled(self.is_scanning)
                            .large(),
                    ),
                ),
            )
            .when(self.is_scanning, |this| {
                this.child(
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
                                .color(cx.theme().colors.foreground),
                        ),
                )
            })
            .when(!self.is_scanning, |this| {
                this.child(
                    div()
                        .id("finder-list")
                        .flex_1()
                        .min_h_0()
                        .overflow_hidden()
                        .child(
                            v_virtual_list(
                                cx.entity().clone(),
                                "my-list",
                                self.item_sizes.clone(),
                                |view, visible_range, _, cx| {
                                    visible_range
                                        .filter_map(|index| {
                                            let item = view.items.get(index)?;
                                            let selected = view.item_selected
                                                == Some(index);

                                            Some(
                                                div()
                                                    .id((
                                                        "finder-result",
                                                        index,
                                                    ))
                                                    .h(px(18.0))
                                                    .w_full()
                                                    .px_3()
                                                    .flex()
                                                    .items_center()
                                                    .when(selected, |view| {
                                                        view.bg(cx
                                                            .theme()
                                                            .colors
                                                            .list_active)
                                                    })
                                                    .child(
                                                        item.clone()
                                                            .path
                                                            .to_string_lossy()
                                                            .to_string(),
                                                    ),
                                            )
                                        })
                                        .collect()
                                },
                            )
                            .track_scroll(&self.scroll_handle),
                        ),
                )
            })
            .child(
                div()
                    .id("finder-status")
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .py_2()
                    .border_t_1()
                    .text_sm()
                    .text_color(cx.theme().colors.muted_foreground)
                    .child(self.search_result.clone()),
            )
    }
}
