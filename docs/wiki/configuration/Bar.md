# Bar

[Wiki](../README.md#configuration) · [Introduction](Introduction.md)

The bar is a top-anchored Wayland layer-shell surface. Without `[bar.*]`
entries, a default bar opens on every connected output, including newly
connected displays. Explicit entries restrict bars to the named outputs.
Configure a separate bar for each output on which it should appear in
`~/.config/ghost-shell/config.toml`:

```toml
[bar."eDP-1"]
output = "eDP-1"
height = 27.0
exclusive_zone = 27.0
appearance = "themed"

[bar."DP-1"]
output = "DP-1"
primary = true
height = 27.0
exclusive_zone = 27.0
appearance = "blur"
background_opacity = 0.8
```

Use `niri msg outputs` to find output names. The **table key**, such as `DP-1`,
is what the current implementation matches to a connected display. The `output`
field exists in the schema but does not select the display; keep it identical to
the table key. Arbitrary table names such as `[bar.main]` will not match `DP-1`.

  | Option               | Default       | Behavior                                                       |
  | -------------------- | ------------- | -------------------------------------------------------------- |
  | `output`             | `"<default>"` | Schema field; output selection currently uses the table key    |
  | `height`             | `27.0`        | Bar height in logical pixels                                   |
  | `exclusive_zone`     | `27.0`        | Space reserved for the bar in logical pixels                   |
  | `primary`            | `false`       | Preferred fallback output within the bar application           |
  | `appearance`         | `"themed"`    | `"themed"` (alias `"default"`), `"blur"`, or `"transparent"`   |
  | `background_opacity` | Automatic     | `1.0` for themed, `0.8` for blur, always `0.0` for transparent |

Themed bars use the active palette's `base00` background without blur. Blur uses
the theme background and asks the compositor to blur behind it. Transparent bars
retain visible widgets without a background or blur.

The primary-output setting is local to the bar application. Launcher and finder
use their own focused-output selection. Bars reconcile when displays change; a
disconnected output's bar is removed and can return on reconnection.

## Widget layout

  | Section | Widgets, in order         |
  | ------- | ------------------------- |
  | Start   | System menu, workspaces   |
  | Center  | Focused window            |
  | End     | Tray, audio, power, clock |

Bar content has a 9px inset on both the left and right edges. The menu,
workspace dots and speaker use 18px icons; the battery uses a 24px state icon.
Icons are vertically centered, with filled and outlined dots marking active and
inactive workspaces respectively. This layout is currently fixed in code. TOML
cannot add, remove, reorder, or independently configure widget instances per
output. Network and notifications are not rendered. See
[widgets](Bar.md#widgets) for current behavior and limits.

Implementation: [bar
manager](../../../crates/ghost-shell-applications/ghost-shell-bar/src/lib.rs),
[layout](../../../crates/ghost-shell-applications/ghost-shell-bar/src/lib.rs),
and [output
selection](../../../crates/ghost-shell-applications/ghost-shell-bar/src/app.rs).

## Widgets

Each implemented widget lives in its own crate under
`crates/ghost-shell-widgets/` and is composed into the bar. The current layout
cannot be changed through TOML, and most widgets have no dedicated
configuration. Their fonts and colors follow the shared theme.

  | Widget                                  | Current state                            | Dedicated configuration |
  | --------------------------------------- | ---------------------------------------- | ----------------------- |
  | [System menu](Menu.md)                  | Launcher shortcut                        | None                    |
  | [Workspaces](Workspaces.md)             | Implemented indicator                    | None                    |
  | [Focused window](Bar.md#focused-window) | Implemented indicator                    | None                    |
  | [System tray](Tray.md)                  | Implemented with incomplete interactions | None                    |
  | [Audio](Audio.md)                       | Implemented controls                     | None                    |
  | [Power](Bar.md#power)                   | Partial battery indicator                | None                    |
  | [Clock](Clock.md)                       | Implemented                              | `[clock].format`        |
  | [Network](Bar.md#network)               | Not implemented                          | None                    |
  | [Notifications](Bar.md#notifications)   | Not implemented as a widget              | None                    |

The linked pages and sections describe behavior, supported configuration, and
limits. Status here describes what exists; the
[roadmap](../../../README.md#roadmap) tracks whether a feature is considered
complete.

Camera, Bluetooth, screenshot/recording, theme-switch, and weather widgets
remain roadmap items without corresponding widget crates. Add reference pages
when an implementation or concrete configuration contract exists.

## Focused window

Status: **Implemented indicator.**

The center of the bar displays the title of Niri's globally focused window and
truncates long titles. With no focused title it displays an empty string. The
same focus entity is shared across bars, so this is not an independent
per-output window title.

### Setup and configuration

Requires the daemon's Niri connection. There are no `[focus]` settings, title
format strings, or click actions. Fonts and colors follow the shared theme.

Implementation: [widget
crate](../../../crates/ghost-shell-widgets/ghost-shell-widget-focus/src/focus.rs).

## Power

Status: **Partial battery indicator.**

Displays a 24px icon for the first enumerated battery using `starship-battery`.
Charging and full batteries use dedicated icons; discharging and paused
batteries use charge-level icons. A missing battery leaves the indicator empty
and does not prevent bars from appearing. Read errors are logged.

### Setup and configuration

There are no `[power]` settings, battery selectors, or power-profile controls.
Battery data is read at startup; periodic refresh remains unimplemented.

Implementation: [widget
crate](../../../crates/ghost-shell-widgets/ghost-shell-widget-power/src/power.rs).

## Network

Status: **Not implemented.**

Wi-Fi/Ethernet status and controls remain roadmap work.

### Setup and configuration

No setup, interactions, or `[network]` options are available yet. Adding a TOML
section will not enable a network widget. Document backend requirements,
controls, and settings here when implemented.

This planned feature has no implementation yet.

## Notifications

Status: **Not implemented as a widget.**

Notifications are not included in the bar. No notification service or center is
started by the bar daemon.

### Setup and configuration

There are no supported notification-widget interactions or `[notifications]`
settings. Document service setup, notification history, popup behavior, and
options here as they become implemented.

This planned feature has no implementation yet.
