# Bar

[Wiki](../README.md#configuration) · [Introduction](Introduction.md)

The bar is a top-anchored Wayland layer-shell surface. Geometry, decoration and
widget dimensions are global across outputs. Use the shared `config.toml`:

```toml
[bar]
height = 28.0
exclusive_zone = 28.0

[bar.outputs."eDP-1"]
primary = true

[bar.outputs."DP-1"]
primary = false

[bar.decoration]
layout = "islands"
opacity = 0.8
blur = true
radius = 4.0

[bar.widgets]
height = 20.0
icon_size = 16.0
padding_horizontal = 4.0
margin_horizontal = 2.0
margin_vertical = 4.0
```

## Outputs and geometry

With no `bar.outputs` entries, bars open on all connected outputs, including
newly connected displays. Otherwise the keys select outputs by exact name from
`niri msg outputs`. Disconnecting an output removes its bar; reconnecting it
restores the bar. There is no redundant `output` field or per-output styling.
The `primary` flag is retained as output metadata; current popups attach to the
bar that was clicked, and launcher/finder use Niri's active output.

| Option | Default | Meaning |
| --- | --- | --- |
| `bar.height` | `28.0` | Bar height in logical pixels |
| `bar.exclusive_zone` | `28.0` | Space reserved for the bar; zero reserves none |
| `bar.outputs.NAME.primary` | `false` | Output metadata |

Choose a bar height at least `widgets.height + 2 * widgets.margin_vertical`.
Choose an icon size that fits inside the widget height. Dimensions must be
finite, nonnegative and no greater than 16384; heights and icon size must be
positive. Invalid settings log a configuration error and use application defaults.

## Decoration

| Option | Default | Meaning |
| --- | --- | --- |
| `layout` | `"continuous"` | `"continuous"` decorates the bar; `"islands"` decorates widget inner containers |
| `opacity` | `1.0` | Background opacity, from zero to one |
| `blur` | `false` | Request compositor blur behind decorated areas |
| `radius` | `4.0` | Decoration corner radius in logical pixels |

Backgrounds use the active palette's `base00`. Text and monochrome icons use
`base07`, and inner containers highlight with `base02` on hover. These colors
are not separate bar options. Opacity affects resting backgrounds, not content
or hover highlights. Original tray icons retain their own colors.

| Appearance | Opacity | Blur |
| --- | --- | --- |
| Solid | `1.0` | `false` |
| Transparent | `0.0` | `false` |
| Translucent | Between zero and one | `false` |
| Blurred with a tint | Below one | `true` |
| Blurred without a tint | `0.0` | `true` |

Both layouts support all combinations. Continuous mode leaves inner containers
transparent at rest. Islands mode leaves the surrounding bar and gaps transparent.
Joined tray/audio/network groups round only their group's outside edges.

Blur requires compositor support; without it the configured background still
renders. Ghost updates explicit surface-local blur regions when decoration
bounds change, including tray changes and output resizing. Rounded corners are
approximated conservatively using integer logical-pixel rectangles, leaving gaps
clear. Blur is not simulated and has no configurable strength. Popups retain
their own styling.

## Widget layout

| Section | Widgets, in order |
| --- | --- |
| Start | System menu, theme toggle, workspaces |
| Center | Focused window |
| End | Tray, notifications, audio, network, power, keyboard layout, clock |

The bar retains 9px horizontal insets. Global widget defaults are:

| `bar.widgets` option | Default |
| --- | --- |
| `height` | `20.0` |
| `icon_size` | `16.0` |
| `padding_horizontal` | `4.0` |
| `margin_horizontal` | `2.0` |
| `margin_vertical` | `4.0` |

Outer containers are transparent and span the bar height. Inner containers
follow content width, with the configured padding and height. Icons preserve
aspect ratio. Tray, audio, and network groups have no internal gap and join at square
edges. Workspaces share one inner container. Empty widgets take no space.
The focused title is constrained to the center section and clipped if too long.
All three sections use these settings. Widget ordering remains fixed.

## Live configuration

The bar watches its configuration directory for file changes, including atomic
editor saves and Home Manager symlink replacements. Events are debounced for
150ms; there is no periodic polling. Reload applies all bar settings, theme
mode and palettes, icon theme, fonts, clock format and wallpaper. Existing bars
resize and change reserved space in place; output selection adds/removes bars.
Tray icon lookup and wallpaper loading run on workers, and stale results cannot
replace newer settings. Blur can be enabled and disabled while running.

An invalid file or removal after a successful load preserves current settings.
The system menu’s **Appearance!** label indicates a reload failure; open it to read the error.
A valid change to bar-relevant file settings replaces the in-memory preview.
The panel's **Reset from file** restores the most recent valid file settings.
Restarting the process discards all unpersisted preview changes.

## Migration

This replaces the old per-output schema; old `[bar."OUTPUT"]` tables are
rejected instead of being interpreted as global settings.

- Move output keys to `[bar.outputs."OUTPUT"]`, retaining only `primary`.
- Move height and exclusive zone to `[bar]`, choosing one value for all outputs.
- Replace `appearance = "themed"` with opacity `1.0`, blur `false`;
  `"transparent"` with opacity `0.0`, blur `false`; and `"blur"` with opacity
  `0.8`, blur `true`. Preserve an explicit old background opacity where applicable.
- Choose `layout = "islands"` for decorated widget containers on a clear bar.
- Move `[general] theme = "Papirus-Dark"` to `[theme] icons = "Papirus-Dark"`.

Home Manager uses the same structure:

```nix
programs.ghost-shell.settings = {
  theme = { mode = "dark"; icons = "Papirus-Dark"; };
  bar = {
    height = 28;
    exclusive_zone = 28;
    outputs."eDP-1".primary = true;
    decoration = { layout = "islands"; opacity = 0.8; blur = true; radius = 4; };
    widgets = {
      height = 20;
      icon_size = 16;
      padding_horizontal = 4;
      margin_horizontal = 2;
      margin_vertical = 4;
    };
  };
};
```

Install the icon theme separately. Valid configuration changes reload live.
Open **Appearance** in the system menu for [live styling](Styling.md).

Implementation: [bar layout](../../../crates/ghost-shell-applications/ghost-shell-bar/src/lib.rs),
[shared decoration](../../../crates/ghost-shell-core/ghost-shell-theme/src/bar.rs),
[blur regions](../../../crates/ghost-shell-applications/ghost-shell-bar/src/blur.rs).

## Widgets

Each implemented widget lives in its own crate under
`crates/ghost-shell-widgets/` and is composed into the bar. The current layout
cannot be changed through TOML, and most widgets have no dedicated
configuration. Their fonts and colors follow the shared theme.

  | Widget                                  | Current state                            | Dedicated configuration |
  | --------------------------------------- | ---------------------------------------- | ----------------------- |
  | [System menu](Menu.md)                  | Shutdown, reboot, lock, styling                        | None                    |
  | [Theme toggle](#theme-toggle) | Live dark/light toggle | None |
  | [Workspaces](Workspaces.md)             | Implemented indicator                    | None                    |
  | [Focused window](Bar.md#focused-window) | Implemented indicator                    | None                    |
  | [System tray](Tray.md)                  | Implemented with incomplete interactions | None                    |
  | [Audio](Audio.md)                       | Implemented controls                     | None                    |
  | [Power](Bar.md#power)                   | Partial battery indicator                | None                    |
  | [Clock](Clock.md)                       | Implemented                              | `[clock].format`        |
  | [Keyboard layout](#keyboard-layout)     | Active language indicator                | None                    |
  | [Network](Bar.md#network)               | Ethernet/Wi-Fi status indicator           | None                    |
  | [Notifications](Notifications.md)       | Unread list and mute control              | None                    |

The linked pages and sections describe behavior, supported configuration, and
limits. Status here describes what exists; the
[roadmap](../../../README.md#roadmap) tracks whether a feature is considered
complete.

Camera, Bluetooth, screenshot/recording, and weather widgets
remain roadmap items without corresponding widget crates. Add reference pages
when an implementation or concrete configuration contract exists.

## Theme toggle

The half-circle icon between the system menu and workspaces switches all bars
and their popups between dark and light palettes. In system mode, the first
click selects the opposite of the currently displayed appearance.

This is an in-memory override, like the Appearance panel. It leaves the saved
configuration and other applications unchanged. Use Appearance → Copy Nix to
persist the selected mode, or Reset from file to restore it (including system
mode). The icon follows the shared bar widget dimensions and decoration.

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

## Keyboard layout

The indicator immediately before the clock displays the active keyboard layout's
lowercase ISO 639-1 language code, such as `en`, `ru`, or `de`. Layout variants
share their language code. Niri supplies XKB display names; the widget removes
parenthesized variant information and looks up the English language name.
Custom names and languages without a two-letter code display `--`.

The existing Niri event subscription supplies the initial layout and subsequent
switches and layout-list changes to all bars. No polling or additional connection
is needed. Missing or invalid layout state hides the widget, including while the
Niri connection is unavailable; reconnecting restores it from the event stream.

There are no dedicated settings or click actions. Configure layouts and switching
in Niri. Fonts, colors, dimensions and decoration follow the shared bar settings,
matching the clock.

Implementation: [widget](../../../crates/ghost-shell-widgets/ghost-shell-widget-keyboard-layout/src/keyboard_layout.rs),
[Niri state](../../../crates/ghost-shell-adapters/ghost-shell-niri/src/protocol.rs).

## Power

Status: **Partial battery indicator.**

Displays a 16px icon for the first enumerated battery using `starship-battery`.
Charging uses `battery-vertical-charging.svg`. A full battery uses
`battery-vertical-charging-2.svg` when an external system power supply reports
online through Linux sysfs, and `battery-vertical-4.svg` otherwise.
Other states select by charge: `battery-vertical-1.svg` below 25%,
`battery-vertical-2.svg` from 25% to below 50%, `battery-vertical-3.svg` from
50% to below 76%, and `battery-vertical-4.svg` at 76% or above. A missing battery leaves the indicator empty
and does not prevent bars from appearing. Read errors are logged.

### Setup and configuration

There are no `[power]` settings, battery selectors, or power-profile controls.
Battery data is read at startup; periodic refresh remains unimplemented.

Implementation: [widget
crate](../../../crates/ghost-shell-widgets/ghost-shell-widget-power/src/power.rs).

## Network

Status: **Implemented indicator.**

The joined Ethernet and Wi-Fi icons sit between audio and power. Both remain
visible: a slash means no activated connection of that type. Ethernet uses
`network-fill.svg` when at least one Ethernet device is activated. Wi-Fi uses
`wifi-low-fill.svg` below 34%, `wifi-medium-fill.svg` from 34–66%, and
`wifi-high-fill.svg` at 67% or above. With multiple active Wi-Fi devices, the
icon represents the strongest signal. An activated Wi-Fi device with no
reported access point uses the low icon.

The icons have no hover tooltips or menu yet. Active means NetworkManager
reports an activated device; it does not
guarantee Internet access. VPNs and other virtual interfaces do not count as
Ethernet or Wi-Fi. This widget only displays status; it does not scan, connect,
disconnect, or change radio settings.

### Setup and configuration

Requires NetworkManager on the system D-Bus. On NixOS, enable
`networking.networkmanager.enable = true;` in the system configuration. No
`[network]` settings are needed. Dimensions, fonts and colors follow the global
bar settings.

One monitor serves all outputs. Updates follow D-Bus signals, including active
access point changes and service restarts. Failed reads clear the active icons
and recovery is automatic. The healthy monitor does
not poll. Failed bus connections or reads retry after five seconds.

Implementation: [widget](../../../crates/ghost-shell-widgets/ghost-shell-widget-network/src/network.rs),
[NetworkManager adapter](../../../crates/ghost-shell-adapters/ghost-shell-dbus/src/networkmanager.rs).

## Notifications

The bell between tray and audio opens the unread notification list; right-click
toggles mute. See [Notifications](Notifications.md) for service setup, dismissal,
timeouts, and limitations.


Menus opened from the start section align their left edge with the triggering
inner container. End-section menus align their right edge with the triggering
inner container, including individual tray icons and audio controls. Menus open
7 logical pixels below the bar's bottom edge. Appearance inherits the system
menu's anchor. Open menus follow widget layout changes and retain alignment
when their content resizes; the compositor can slide them to stay on screen.

Widget menus (including tray submenus, audio, notifications, system menu, and
Appearance) inherit `bar.decoration.opacity` and `bar.decoration.blur`. Open
menus update live. Blur follows each rounded menu background rather than the
entire popup rectangle, leaving empty areas clear. At opacity `1.0`, the opaque
background hides blur; use a value below `1.0` to see it. The compositor must
support the background-effect protocol. Menu corner radii remain unchanged.
