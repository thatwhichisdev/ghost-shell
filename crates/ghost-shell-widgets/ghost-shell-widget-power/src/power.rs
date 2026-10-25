use iced::{Element, widget::Space};
use starship_battery::{State, units::ratio::percent};

#[derive(Default)]
pub struct Power {
    icon: Option<&'static str>,
}

impl Power {
    pub fn update(&mut self, icon: Option<&'static str>) {
        self.icon = icon;
    }

    pub fn view<'a, Message: 'a>(&'a self) -> Element<'a, Message> {
        match self.icon {
            Some(icon) => ghost_shell_assets::icon(icon, 24.0),
            None => Space::new().into(),
        }
    }
}

pub fn battery() -> Result<Option<&'static str>, String> {
    let manager = starship_battery::Manager::new().map_err(|error| error.to_string())?;
    let mut batteries = manager
        .batteries()
        .map_err(|error| error.to_string())?;
    let Some(battery) = batteries.next() else {
        return Ok(None);
    };
    let battery = battery.map_err(|error| error.to_string())?;
    let icon = match battery.state() {
        State::Full => "icons/battery-full.svg",
        State::Charging => "icons/battery-charging.svg",
        State::Unknown | State::Empty => "icons/battery-empty.svg",
        State::Discharging | State::Paused => {
            match battery.state_of_charge().get::<percent>() {
                level if level <= 25.0 => "icons/battery-25.svg",
                level if level <= 50.0 => "icons/battery-50.svg",
                level if level <= 75.0 => "icons/battery-75.svg",
                _ => "icons/battery-full.svg",
            }
        }
    };
    Ok(Some(icon))
}
