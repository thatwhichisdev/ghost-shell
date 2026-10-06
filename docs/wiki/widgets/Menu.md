# System menu

[Widgets](../Widgets.md) · [Bar](../Bar.md) · [Wiki](../README.md)

Status: **Menu icon placeholder.**

The start of the bar shows a NixOS icon. It is currently a visual placeholder:
there is no click handler, application menu, or session/power menu.

## Setup and configuration

The icon is included in the fixed bar layout. There are no `[menu]` settings or
configurable icon paths. Use the [CLI](../Commands.md) or Niri bindings to open
the launcher and lock the session.

Implementation: [widget source](../../../crates/ghost-shell-widgets/ghost-shell-widget-menu/src/menu.rs).
