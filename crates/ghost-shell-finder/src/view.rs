use gpui::{
    Context, Entity, IntoElement, Render, Task, Window, div, prelude::*,
};
use gpui_component::{
    ActiveTheme as _, IconName, IndexPath, Sizable,
    form::field,
    input::{Input, InputEvent, InputState},
    list::{List, ListItem, ListState},
    spinner::Spinner,
};

use crate::search::{Search, SearchItem, SearchOptions};

pub(crate) struct View {
    input_path: Entity<InputState>,
    input_query: Entity<InputState>,
    list: Entity<ListState<ListDelegate>>,

    search: Option<Search>,
    searching: bool,
    search_task: Task<()>,
}

pub(crate) struct ListDelegate {
    items: Vec<SearchItem>,
    index: Option<IndexPath>,
    is_loading: bool,
}

impl View {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_path =
            cx.new(|cx| InputState::new(window, cx).default_value("/"));

        let input_query = cx.new(|cx| {
            let input = InputState::new(window, cx);

            input.focus(window, cx);
            input
        });

        let list = cx.new(|cx| {
            let delegate = ListDelegate {
                items: vec![],
                index: None,
                is_loading: false,
            };

            ListState::new(delegate, window, cx)
        });

        cx.subscribe_in(&input_query, window, Self::on_query_event)
            .detach();

        Self {
            input_path,
            input_query,
            list,
            search: None,
            searching: false,
            search_task: Task::ready(()),
        }
    }

    fn on_query_event(
        &mut self,
        input: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let InputEvent::PressEnter {
            secondary: false,
            shift: false,
        } = event
        else {
            return;
        };

        self.start_search(input, window, cx);
    }

    fn start_search(
        &mut self,
        input: &Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.searching {
            return;
        }

        let path_value = self.input_path.read(cx).value();
        let base_path = path_value.trim().to_owned();

        let query_value = input.read(cx).value();
        let needle = query_value.trim().to_owned();

        if base_path.is_empty() || needle.is_empty() {
            return;
        }

        let needs_index = self.search.as_ref().is_none_or(|search| {
            search.base_path() != std::path::Path::new(&base_path)
        });

        self.searching = true;

        self.list.update(cx, |list, cx| {
            list.delegate_mut().is_loading = true;
            cx.notify();
        });

        // Results from another root are no longer valid, so clear them while
        // the new root is being indexed.
        if needs_index {
            self.list.update(cx, |list, cx| {
                list.delegate_mut().items.clear();
                list.set_selected_index(None, window, cx);
                cx.notify();
            });
        }

        // Transfer ownership to the background operation. It will be returned
        // to View when the operation completes.
        let existing_search = self.search.take();

        let background_task = cx.background_spawn(async move {
            let search = match existing_search {
                Some(search)
                    if search.base_path()
                        == std::path::Path::new(&base_path) =>
                {
                    search
                }

                _ => {
                    let mut search = Search::try_new(SearchOptions {
                        base_path,
                        enable_content_indexing: false,
                    })?;

                    search.index()?;

                    search
                }
            };

            let items = search.search(&needle, 100);

            anyhow::Ok((search, items))
        });

        self.search_task = cx.spawn_in(window, async move |this, cx| {
            let result = background_task.await;

            let _ = this.update_in(cx, |this, window, cx| {
                this.searching = false;

                this.input_query.update(cx, |input, cx| {
                    input.set_loading(false, window, cx);
                });

                match result {
                    Ok((search, items)) => {
                        this.search = Some(search);

                        this.list.update(cx, |list, cx| {
                            let first_item = (!items.is_empty())
                                .then_some(IndexPath::default());

                            list.delegate_mut().items = items;
                            list.delegate_mut().is_loading = false;

                            list.set_selected_index(first_item, window, cx);

                            cx.notify();
                        });
                    }

                    Err(error) => {
                        eprintln!("Failed to perform finder search: {error:#}");
                    }
                }
            });
        });
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
            .overflow_hidden()
            .flex()
            .flex_col()
            .bg(cx.theme().colors.background)
            .text_color(cx.theme().colors.foreground)
            .child(div().id("finder-input-path").p_3().border_b_1().child(
                field().label("Base path").child(
                    Input::new(&self.input_path).large().cleanable(true),
                ),
            ))
            .child(div().id("finder-input-query").p_3().border_b_1().child(
                field().label("Search query").child(
                    Input::new(&self.input_query).large().cleanable(true),
                ),
            ))
            .child(
                div()
                    .id("finder-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(List::new(&self.list).size_full()),
            )
    }
}

impl gpui_component::list::ListDelegate for ListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &gpui::App) -> usize {
        self.items.len()
    }

    fn loading(&self, _cx: &gpui::App) -> bool {
        self.is_loading
    }

    fn render_loading(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .h_full()
            .w_full()
            .child(
                Spinner::new()
                    .large()
                    .icon(IconName::LoaderCircle)
                    .color(cx.theme().colors.foreground),
            )
    }

    fn render_item(
        &mut self,
        index: IndexPath,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        self.items.get(index.row).map(|item| {
            ListItem::new(index)
                .truncate()
                .text_sm()
                .child(item.clone().path.to_string_lossy().to_string())
                .selected(Some(index) == self.index)
        })
    }

    fn set_selected_index(
        &mut self,
        index: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.index = index;
        cx.notify();
    }
}
