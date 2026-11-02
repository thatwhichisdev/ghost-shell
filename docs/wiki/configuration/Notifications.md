# Notifications

[Wiki](../README.md#configuration) · [Bar](Bar.md)

The bell appears between tray and audio and follows the shared bar styling.

- Left-click opens or closes the unread list.
- Right-click toggles mute. Notifications continue to collect while muted;
  the slashed bell replaces the ringing indicator.
- Each entry shows an app icon (or fallback), app name, summary, optional body,
  and **Mark as read**. Reading dismisses the entry; opening the list does not.
- Escape or an outside click closes the list. It shares the popup slot with
  tray, audio, system, and Appearance menus.

## Setup

The bar hosts `org.freedesktop.Notifications` on the session bus. Stop and disable
other notification daemons, such as mako or dunst, before starting the bar.
Ghost will not replace another daemon: a conflict is shown in the menu. After
resolving a conflict, restart the bar to start its notification service.

There are no additional notification settings. Mute and unread entries live in
memory and reset when the bar restarts. They are shared by all bar outputs.

With `notify-send` installed, test delivery using:

```sh
notify-send --app-name="Ghost test" --icon=dialog-information --expire-time=0 \
  "Hello from Ghost" "Click the bell, then mark this notification as read."
```

## Protocol behavior

The service implements the freedesktop notification methods, unique nonzero IDs,
replacement of existing IDs, and `NotificationClosed`. Mark as read emits reason
2 (user dismissed); sender closure emits reason 3; expiration emits reason 1.
Replacement resets the deadline and moves the entry to the newest position.

Default (`-1`) and zero timeouts retain the entry until dismissed or closed by
the sender. Positive timeouts expire in milliseconds. The `transient` hint with
a default timeout uses five seconds. At most 256 entries are retained; new
notifications beyond that limit return an error, while replacements remain
allowed. Text payloads are limited to 64 KiB.

Capabilities are `body` and `persistence`. This first version is an inbox: it
does not display automatic toasts or play sounds. App-provided actions, body
markup, hyperlinks, and embedded image hints are unsupported and not advertised.
Body text is displayed literally; clients should honor the capabilities.

Icons resolve from the supplied icon name or absolute file path, with
`desktop-entry` fallback through installed desktop files. Lookup uses
`theme.icons` and refreshes when it changes. Missing icons use a bell fallback;
application icons retain their original colors.

Protocol reference: [Desktop Notifications Specification](https://specifications.freedesktop.org/notification/latest/protocol.html).

Implementation: [D-Bus backend](../../../crates/ghost-shell-adapters/ghost-shell-dbus/src/zbus_backend/notifications.rs),
[widget](../../../crates/ghost-shell-widgets/ghost-shell-widget-notifications/src/notifications.rs),
[bar services](../../../crates/ghost-shell-applications/ghost-shell-bar/src/services.rs).
