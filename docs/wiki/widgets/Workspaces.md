# Workspaces

[Widgets](../Widgets.md) · [Bar](../Bar.md) · [Wiki](../README.md)

Status: **Implemented indicator.**

Displays workspaces belonging to the bar's output, sorted by workspace index.
The active workspace uses a filled circle; other workspaces use an outline.
Updates come from Niri's state stream.

## Setup and configuration

Configure the bar for the correct output in the [bar guide](../Bar.md).
There are no `[workspaces]` settings. The indicator is display-only; it currently
has no click handler for switching workspaces. Use your Niri workspace bindings.

Implementation: [widget source](../../../crates/ghost-shell-widgets/ghost-shell-widget-workspaces/src/workspaces.rs).
