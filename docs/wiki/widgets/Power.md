# Power

[Widgets](../Widgets.md) · [Bar](../Bar.md) · [Wiki](../README.md)

Status: **Partial battery indicator.**

Displays a 24px icon for the first enumerated battery using
`starship-battery`. Charging and full batteries use
dedicated icons; discharging and paused batteries use charge-level icons. A missing
battery leaves the indicator empty and does not prevent bars from appearing.
Read errors are logged.

## Setup and configuration

There are no `[power]` settings, battery selectors, or power-profile controls.
Battery data is read at startup; periodic refresh remains unimplemented.

Implementation: [widget crate](../../../crates/ghost-shell-widgets/ghost-shell-widget-power/src/power.rs).
