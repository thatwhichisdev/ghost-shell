use gpui::{IntoElement, div, prelude::*};

pub fn mock_widget(id: &'static str, label: &'static str) -> impl IntoElement {
    div().id(id).flex().items_center().px_2().child(label)
}
