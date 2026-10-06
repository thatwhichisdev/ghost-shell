# Widgets

[Wiki](README.md) · [Bar layout](Bar.md)

Widgets are compiled into the bar. The current layout cannot be changed through
TOML, and most widgets have no dedicated configuration. Their fonts and colors
follow the shared theme.

| Widget | Current state | Dedicated configuration |
| --- | --- | --- |
| [System menu](widgets/Menu.md) | Menu icon placeholder | None |
| [Workspaces](widgets/Workspaces.md) | Implemented indicator | None |
| [Focused window](widgets/Focus.md) | Implemented indicator | None |
| [System tray](widgets/Tray.md) | Implemented with incomplete interactions | None |
| [Audio](widgets/Audio.md) | Implemented controls | None |
| [Power](widgets/Power.md) | Partial battery indicator | None |
| [Clock](widgets/Clock.md) | Implemented | `[clock].format` |
| [Network](widgets/Network.md) | Not implemented | None |
| [Notifications](widgets/Notifications.md) | Not implemented as a widget | None |

Each page describes behavior, setup, supported configuration, and current limits.
Status here describes what exists; the [roadmap](../README.md#roadmap) tracks
whether a feature is considered complete.

Camera, Bluetooth, screenshot/recording, theme-switch, and weather widgets remain
roadmap items without corresponding widget crates. Add reference pages when an
implementation or concrete configuration contract exists.
