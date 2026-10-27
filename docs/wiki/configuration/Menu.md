# Menu

[Wiki](../README.md#configuration) · [Introduction](Introduction.md) ·
[Bar](Bar.md)

Status: **Launcher shortcut.**

The start of the bar shows an 18px NixOS icon, centered vertically. Click it to
launch the application launcher through Niri. A session/power menu is not
implemented.

## Setup and configuration

The icon is included in the fixed bar layout. There are no `[menu]` settings or
configurable icon paths. Bind `ghost-shell-launcher` directly in Niri to open
the launcher. Use the [CLI](../getting-started/Installation.md#commands-and-ipc)
for session locking.

Implementation: [widget
crate](../../../crates/ghost-shell-widgets/ghost-shell-widget-menu/src/menu.rs).
