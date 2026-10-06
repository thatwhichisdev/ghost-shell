# Audio

[Widgets](../Widgets.md) · [Bar](../Bar.md) · [Wiki](../README.md)

Status: **Implemented controls.**

Shows the default output's audio status. Click the widget to open its panel,
which provides output and input volume sliders and device lists. Select a device
to make it the default for its direction. Escape closes the panel.

## Setup and configuration

Requires a working PipeWire session. Devices and defaults are discovered through
the audio backend; there are no `[audio]` settings for device names or volumes.
The panel follows the shared theme.

## Current limitations

Sliders cover 0–100%; amplification above 100% is not exposed. Muted status is
shown, but the widget does not currently provide a dedicated mute toggle.
Unavailable connections and failed commands are reported in the panel. Controls
are disabled when the relevant device or connection is unavailable.

Implementation: [widget source](../../../crates/ghost-shell-widgets/ghost-shell-widget-audio/src/audio.rs).
