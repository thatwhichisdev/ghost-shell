use anyhow::{Context as _, Result};
use gpui::{Context, Window, div, prelude::*, px};
use gpui_component::{ActiveTheme, Icon, Sizable};
use starship_battery::{Battery, Manager, State, units::ratio::percent};

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

        // todo: implement a task that will check battery status every minute

        Ok(Self { battery })
    }

    fn battery_icon(&self) -> &'static str {
        match self.battery.state() {
            State::Full => "icons/battery-full.svg",
            State::Charging => "icons/battery-charging.svg",
            State::Unknown | State::Empty => "icons/battery-empty.svg",
            State::Discharging => {
                match self.battery.state_of_charge().get::<percent>() {
                    level if level <= 25.0 => "icons/battery-25.svg",
                    level if level <= 50.0 => "icons/battery-50.svg",
                    level if level <= 75.0 => "icons/battery-75.svg",
                    _ => "icons/battery-full.svg",
                }
            }
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
                .with_size(px(24.0))
                .text_color(cx.theme().colors.foreground),
        )
    }
}
