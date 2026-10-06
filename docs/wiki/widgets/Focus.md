# Focused window

[Widgets](../Widgets.md) · [Bar](../Bar.md) · [Wiki](../README.md)

Status: **Implemented indicator.**

The center of the bar displays the title of Niri's globally focused window and
truncates long titles. With no focused title it displays an empty string.
The same focus entity is shared across bars, so this is not an independent
per-output window title.

## Setup and configuration

Requires the daemon's Niri connection. There are no `[focus]` settings, title
format strings, or click actions. Fonts and colors follow the shared theme.

Implementation: [widget source](../../../crates/ghost-shell-widgets/ghost-shell-widget-focus/src/focus.rs).
