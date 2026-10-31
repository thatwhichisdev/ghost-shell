use std::time::Duration;

use futures::SinkExt as _;
use iced::{Element, Subscription, widget::text};

#[derive(Default)]
pub struct Clock {
    label: String,
}
impl Clock {
    pub fn update(&mut self, now: &jiff::Zoned, format: &str) {
        self.label = now.strftime(format).to_string();
    }
    pub fn view<Message>(&self, font: iced::Font, size: f32) -> Element<'_, Message> {
        text(&self.label)
            .font(font)
            .size(size)
            .into()
    }
}

pub fn subscription(format: &str) -> Subscription<()> {
    Subscription::run_with(
        if ["%S", "%T", "%r", "%X", "%c", "%s", "%f", "%N"]
            .iter()
            .any(|directive| format.contains(directive))
        {
            1
        } else {
            60
        },
        ticks,
    )
}

fn ticks(interval: &u64) -> impl futures::Stream<Item = ()> + use<> {
    let interval = Duration::from_secs(*interval);
    iced::stream::channel(1, async move |mut output| {
        loop {
            let elapsed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let remaining =
                interval.as_nanos() - elapsed.as_nanos() % interval.as_nanos();
            tokio::time::sleep(Duration::from_nanos(remaining as u64)).await;
            if output.send(()).await.is_err() {
                break;
            }
        }
    })
}
