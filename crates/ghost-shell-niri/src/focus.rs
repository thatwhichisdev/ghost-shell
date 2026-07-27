use gpui::{Context, SharedString, Window, div, prelude::*};
use tokio::sync::broadcast;

use crate::client::state::NiriState;

pub struct Focus {
    title: SharedString,
}

impl Focus {
    #[must_use]
    pub fn new(
        cx: &mut Context<Self>,
        mut receiver: broadcast::Receiver<NiriState>,
    ) -> Self {
        // Spawn a task that will update title when focused window changes
        cx.spawn(async move |focus, cx| {
            loop {
                if let Ok(state) = receiver.recv().await {
                    focus
                        .update(cx, |clock, cx| {
                            if let Some(window) = state
                                .windows
                                .into_iter()
                                .find(|window| window.1.is_focused == true)
                            {
                                clock.title = window
                                    .1
                                    .title
                                    .map(|title| title.to_string())
                                    .unwrap_or("".to_owned())
                                    .into();

                                cx.notify();
                            }
                        })
                        .unwrap();
                }
            }
        })
        .detach();

        Self {
            title: Default::default(),
        }
    }
}

impl Render for Focus {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().id("focus").truncate().child(self.title.clone())
    }
}
