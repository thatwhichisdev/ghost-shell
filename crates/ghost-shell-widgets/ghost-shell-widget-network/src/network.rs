use ghost_shell_dbus::networkmanager::Snapshot;
use ghost_shell_theme::bar::{self, Style};
use iced::{Element, widget::row};

#[derive(Default)]
pub struct Network {
    snapshot: Option<Snapshot>,
    error: Option<String>,
}

impl Network {
    pub fn update(&mut self, update: Result<Snapshot, String>) {
        match update {
            Ok(snapshot) => {
                self.snapshot = Some(snapshot);
                self.error = None;
            }
            Err(error) => {
                self.snapshot = None;
                self.error = Some(error);
            }
        }
    }

    pub fn view<Message: 'static>(&self, style: Style) -> Element<'_, Message> {
        let wired_active = self
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| !snapshot.wired.is_empty());
        let wireless = self.snapshot.as_ref().and_then(|snapshot| {
            snapshot
                .wireless
                .iter()
                .max_by_key(|connection| connection.strength)
        });
        let wired_icon = if wired_active {
            "icons/network-fill.svg"
        } else {
            "icons/network-slash-fill.svg"
        };
        let wireless_icon = wifi_icon(wireless.map(|connection| connection.strength));
        let item = |icon, first, last| {
            bar::group_inner(bar::icon(icon, style.widgets.icon_size), first, last, style)
        };
        bar::outer(
            bar::group(
                row([
                    item(wired_icon, true, false),
                    item(wireless_icon, false, true),
                ])
                .spacing(0)
                .align_y(iced::Center),
                style,
            ),
            style,
        )
    }
}

fn wifi_icon(strength: Option<Option<u8>>) -> &'static str {
    match strength {
        None => "icons/wifi-slash-fill.svg",
        Some(Some(67..)) => "icons/wifi-high-fill.svg",
        Some(Some(34..=66)) => "icons/wifi-medium-fill.svg",
        Some(_) => "icons/wifi-low-fill.svg",
    }
}

#[cfg(test)]
mod tests {
    use ghost_shell_dbus::networkmanager::WirelessConnection;

    use super::*;

    #[test]
    fn signal_levels_keep_zero_and_unknown_connected() {
        assert_eq!(wifi_icon(None), "icons/wifi-slash-fill.svg");
        for strength in [None, Some(0), Some(33)] {
            assert_eq!(wifi_icon(Some(strength)), "icons/wifi-low-fill.svg");
        }
        for strength in [34, 66] {
            assert_eq!(
                wifi_icon(Some(Some(strength))),
                "icons/wifi-medium-fill.svg"
            );
        }
        for strength in [67, 100] {
            assert_eq!(wifi_icon(Some(Some(strength))), "icons/wifi-high-fill.svg");
        }
    }

    #[test]
    fn failure_clears_connections_and_recovery_clears_error() {
        let mut network = Network::default();
        network.update(Ok(Snapshot {
            wired: vec!["eth0".into()],
            wireless: vec![WirelessConnection {
                interface: "wlan0".into(),
                ssid: "Home".into(),
                strength: Some(80),
            }],
        }));
        assert!(network.snapshot.is_some());
        network.update(Err("Service stopped".into()));
        assert!(network.snapshot.is_none());
        assert!(network.error.is_some());
        network.update(Ok(Snapshot::default()));
        assert!(network.error.is_none());
        assert!(network.snapshot.is_some());
    }
}
