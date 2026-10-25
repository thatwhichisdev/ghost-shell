# Audio

[Widgets](../Widgets.md) · [Bar](../Bar.md) · [Wiki](../README.md)

Status: **Implemented controls.**

Shows an 18px speaker icon and the default output's audio status. Click the widget to open its panel,
which provides output and input volume sliders and device lists. Select a device
to make it the default for its direction. Escape closes the panel.

## Setup and configuration

Requires a working PipeWire session. Devices and defaults are discovered through
the audio backend; there are no `[audio]` settings for device names or volumes.
The panel follows the shared theme.

## Current limitations

Sliders cover 0–100%; amplification above 100% is not exposed. Each endpoint with
mute support provides a Mute/Unmute button.
Unavailable connections and failed commands are reported in the panel. Controls
are built from the available endpoint capabilities; rejected backend commands
show an error.

Implementation: [widget crate](../../../crates/ghost-shell-widgets/ghost-shell-widget-audio/src/audio.rs).
