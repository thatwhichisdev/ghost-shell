use std::time::Duration;

use gpui::{Context, SharedString, Window, div, prelude::*, px};
use jiff::Zoned;

use ghost_shell_config::AppConfig;

pub struct ClockWidget {
    time: SharedString,
}

impl ClockWidget {
    #[must_use]
    pub fn new(cx: &mut Context<Self>) -> Self {
        let config = cx.global::<AppConfig>().clock.clone();
        let time = formatted_time(&config.format);

        // Spawn a task that will update clock's state every 60 seconds
        cx.spawn(async move |clock, cx| {
            loop {
                cx.background_executor().timer(Duration::from_mins(1)).await;

                if let Err(err) = clock.update(cx, |clock, cx| {
                    clock.time = formatted_time(&config.format);
                    cx.notify();
                }) {
                    eprintln!("Failed to update clock widget state {err:#}");
                }
            }
        })
        .detach();

        Self { time }
    }
}

impl Render for ClockWidget {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id("clock")
            .flex()
            .items_center()
            .pr(px(8.0))
            .child(self.time.clone())
    }
}

fn formatted_time(format: &str) -> SharedString {
    Zoned::now().strftime(format).to_string().into()
}
