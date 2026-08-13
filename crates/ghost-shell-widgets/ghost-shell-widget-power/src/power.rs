use anyhow::{Context as _, Result};
use gpui::{Context, Window, div, prelude::*, px};
use gpui_component::{ActiveTheme, Icon, Sizable};
use starship_battery::{Battery, Manager, State};

pub struct PowerWidget {
    battery: Battery,
}

impl PowerWidget {
    pub fn try_new() -> Result<Self> {
        let manager = Manager::new()?;

        let battery = manager
            .batteries()?
            .filter_map(Result::ok)
            .find(|battery| battery.model().is_some())
            .context("no suitable battery found")?;

        Ok(Self { battery })
    }

    fn battery_icon(&self) -> &'static str {
        match self.battery.state() {
            State::Unknown => "icons/battery-absent.svg",
            State::Charging => "icons/battery-charging.svg",
            State::Discharging => "icons/battery-charging.svg",
            State::Empty => "icons/battery-empty.svg",
            State::Full => "icons/battery-full.svg",
        }
    }
}

impl Render for PowerWidget {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let icon = self.battery_icon();

        div().id("power").flex().items_center().child(
            Icon::empty()
                .path(icon)
                .with_size(px(18.0))
                .text_color(cx.theme().colors.foreground),
        )
    }
}
