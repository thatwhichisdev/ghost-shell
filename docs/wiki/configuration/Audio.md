# Audio

[Wiki](../README.md#configuration) · [Introduction](Introduction.md) ·
[Bar](Bar.md)

Status: **Implemented controls.**

The bar shows two 16px icons: microphone first, speaker second. Each has its
own padded hover container. They join without a gap, with rounded left corners
on the microphone and rounded right corners on the speaker. There is no volume
text beside the icons. Click the microphone for input devices or the speaker
for output devices; each opens an iced_aw menu in a bar-owned Wayland popup.

The microphone uses `mic.svg`, or `mic-off.svg` when the default input is muted.
The speaker uses `volume.svg` at zero volume, `volume-1.svg` above zero and below
50%, and `volume-2.svg` at 50% or higher. Muting the default output overrides
its volume icon with `volume-off.svg`. Without a known default or volume,
the controls use `mic.svg` and `volume-2.svg`; connection and device availability
are reported in their menus.

## Device menu

Each menu lists only devices in its selected direction, without heading rows.
Each device forms a group: its clickable name, then an `M` mute button on the
left and its volume slider on the right. Groups are separated by thin lines
centered in 15px rows. The `M` button is
selected while muted; clicking it toggles mute without closing the menu. Its
selection follows PipeWire's confirmed state. The current default in each direction uses the
selected-row background without adding a prefix to its name. Clicking a name
requests that device as the default; selection follows PipeWire's confirmed
state. A failed request leaves the current selection unchanged and reports the
error in the menu. Changing the default does not guarantee that existing streams
move; routing is controlled by the session manager.

The menu width follows the widest rendered device name plus padding, with a
100px content minimum for the controls and a 760px content maximum for long
names. Longer names wrap. The menu grows and shrinks with its content,
including device additions, removals, and wrapped names. Long lists scroll within the menu's 702px height
limit. The bar owns popup resizing and anchors the menu toward the left, as with
tray menus. Changing mute, volume, or the default device keeps the menu open. Escape,
clicking outside, or clicking the same icon again closes it. Clicking the other
icon switches to its device menu.

Styling follows the tray: `base00` background, `base03` outer border, 6px panel
corners, and device rows with a minimum 30px height and a 4px inset. Active and
hovered name rows use `base07` fill and border with `base00` text and 4px
corners. Names wrap and remain vertically centered. Slider filled tracks and
handles use `base07` in idle, hovered, and dragged states; unfilled tracks use
`base03`. The audio widget owns its menu adapter independently of the tray
widget.

## Setup and configuration

Requires a working PipeWire session. Devices and defaults are discovered through
the audio backend; there are no `[audio]` settings for device names or volumes.
Appearance follows the shared theme.

## Current limitations

Sliders cover 0–100%; amplification above 100% is not exposed. Devices without a
reported volume show `Volume unavailable` instead of a slider. Empty input or
output lists and unavailable connections have explicit status messages.

The mute button is disabled when a device does not report its mute state.
Both bar icons reflect their default device's mute state. Backend command
failures are displayed without changing the confirmed state locally.

Implementation:
[widget](../../../crates/ghost-shell-widgets/ghost-shell-widget-audio/src/audio.rs)
and [menu
adapter](../../../crates/ghost-shell-widgets/ghost-shell-widget-audio/src/menu_popup.rs).
