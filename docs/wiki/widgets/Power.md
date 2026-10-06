# Power

[Widgets](../Widgets.md) · [Bar](../Bar.md) · [Wiki](../README.md)

Status: **Partial battery indicator.**

Displays a battery icon representing charging state and charge level using
`starship-battery`. The current implementation selects the first successfully enumerated battery with a model
value. Charging and charge level determine the icon.

## Setup and configuration

There are no `[power]` settings, battery selectors, or power-profile controls.
A suitable battery must be present for initialization to succeed.

## Current limitations

Battery data is read at creation; periodic refresh is still unimplemented.
If battery initialization fails, the current bar manager logs
`Failed to initialize bar power widget` and returns before creating any bars.
This can prevent the bar from appearing on desktop machines without batteries.
There is no TOML option to bypass the power widget.

Implementation: [widget source](../../../crates/ghost-shell-widgets/ghost-shell-widget-power/src/power.rs).
