# System tray

[Widgets](../Widgets.md) · [Bar](../Bar.md) · [Wiki](../README.md)

Status: **Implemented with incomplete interactions.**

Displays applications registered through the session's StatusNotifier D-Bus
integration. Left-click an item to open its exported menu when it provides one.

## Setup and configuration

Run tray applications in the same session D-Bus environment as Ghost Shell.
The tray is included automatically in the end section of each bar; there are no
`[tray]` options for ordering, filtering, or sizing.

## Current limitations

The current widget wires left-click to a D-Bus menu when present. It does not
provide general activation, secondary-click, or scroll actions for items without
that menu. Application support therefore depends on the menu the item exports.

Implementation: [widget source](../../../crates/ghost-shell-widgets/ghost-shell-widget-tray/src/tray.rs).
