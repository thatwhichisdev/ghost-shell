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

    pub fn view<'a, Message: 'a>(
        &'a self,
        style: ghost_shell_theme::bar::Style,
    ) -> Element<'a, Message> {
        match self.icon {
            Some(icon) => ghost_shell_theme::bar::outer(
                ghost_shell_theme::bar::inner(
                    ghost_shell_theme::bar::icon(icon, style.widgets.icon_size),
                    style,
                ),
                style,
            ),
            None => Space::new().width(0).height(0).into(),
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
        State::Full
            if external_power_connected().map_err(|error| error.to_string())? =>
        {
            "icons/battery-plus-vertical-fill.svg"
        }
        State::Full => "icons/battery-vertical-full-fill.svg",
        State::Charging => "icons/battery-vertical-charging-fill.svg",
        State::Unknown | State::Empty | State::Discharging | State::Paused => {
            match battery.state_of_charge().get::<percent>() {
                level if level < 25.0 => "icons/battery-vertical-low-fill.svg",
                level if level < 50.0 => "icons/battery-vertical-medium-fill.svg",
                level if level < 76.0 => "icons/battery-vertical-high-fill.svg",
                _ => "icons/battery-vertical-full-fill.svg",
            }
        }
    };
    Ok(Some(icon))
}

fn external_power_connected() -> std::io::Result<bool> {
    // Battery status alone does not confirm that an external supply is online.
    for supply in std::fs::read_dir("/sys/class/power_supply")? {
        let path = supply?.path();
        let kind = std::fs::read_to_string(path.join("type"))?;
        if kind.trim() == "Battery" {
            continue;
        }
        match std::fs::read_to_string(path.join("scope")) {
            Ok(scope) if scope.trim() == "Device" => continue,
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        match std::fs::read_to_string(path.join("online")) {
            Ok(online) if online.trim() == "1" => return Ok(true),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(false)
}
