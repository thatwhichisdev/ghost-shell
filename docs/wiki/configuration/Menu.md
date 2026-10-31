# System menu

[Wiki](../README.md#configuration) · [Bar](Bar.md)

Click the system icon at the start of the bar to open these actions:

- **Shutdown** powers off through `systemctl poweroff`.
- **Reboot** restarts through `systemctl reboot`.
- **Lock** requests the running Ghost lockscreen through its IPC endpoint.
- **Appearance** opens the [live styling panel](Styling.md).

Shutdown and reboot run immediately when selected, subject to system permissions
and inhibitors. The bar does not request a terminal password. Failures appear in
the menu, and actions are disabled while a request is pending. Lock requires the
lockscreen service to be running.

Escape or an outside click dismisses the menu. It shares the popup slot with
audio, tray menus, and styling. Opening Appearance closes the menu before creating
the styling popup on the same bar. The system icon no longer launches the
launcher; launch it through your Niri binding.

The menu shares the tray menu’s colors, border, corner radii, row padding, and
hover highlights. Its height follows the displayed content, including feedback.

There are no dedicated system-menu configuration fields.

Implementation: [widget](../../../crates/ghost-shell-widgets/ghost-shell-widget-menu/src/menu.rs),
[bar runtime](../../../crates/ghost-shell-applications/ghost-shell-bar/src/app.rs).
