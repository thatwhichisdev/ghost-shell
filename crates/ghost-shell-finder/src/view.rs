use gpui::{Context, Entity, IntoElement, Render, Window, div, prelude::*};
use gpui_component::{
    ActiveTheme as _, IndexPath, Sizable,
    form::field,
    input::{Input, InputState},
    label::Label,
    list::{List, ListItem, ListState},
};

use crate::search::SearchItem;

pub(crate) struct View {
    input_path: Entity<InputState>,
    input_query: Entity<InputState>,
    list: Entity<ListState<ListDelegate>>,
}

pub(crate) struct ListDelegate {
    items: Vec<SearchItem>,
    index: Option<IndexPath>,
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
            };

            ListState::new(delegate, window, cx)
        });

        Self {
            input_path,
            input_query,
            list,
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
                    .child(List::new(&self.list).size_full()),
            )
    }
}

impl gpui_component::list::ListDelegate for ListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &gpui::App) -> usize {
        self.items.len()
    }

    fn render_item(
        &mut self,
        index: IndexPath,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        self.items.get(index.row).map(|item| {
            ListItem::new(index)
                .child(Label::new(
                    item.clone().path.to_string_lossy().to_string(),
                ))
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
