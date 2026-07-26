use std::time::Duration;

use gpui::{Context, SharedString, Task, Window, div, prelude::*};
use jiff::Zoned;

pub struct Clock {
    time: SharedString,
    _refresh_task: Task<()>,
}

impl Clock {
    #[must_use]
    pub fn new(cx: &mut Context<Self>) -> Self {
        let refresh_task = cx.spawn(async move |clock, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(60))
                    .await;

                if let Err(err) = clock.update(cx, |clock, cx| {
                    clock.time = formatted_time();
                    cx.notify();
                }) {
                    eprintln!("Failed to update clock widget state {err:#}");
                }
            }
        });

        Self {
            time: formatted_time(),
            _refresh_task: refresh_task,
        }
    }
}

impl Render for Clock {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id("clock")
            .flex()
            .items_center()
            .px_2()
            .child(self.time.clone())
    }
}

fn formatted_time() -> SharedString {
    Zoned::now().strftime("%H:%M").to_string().into()
}
