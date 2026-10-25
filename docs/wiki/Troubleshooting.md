# Troubleshooting

[Wiki](README.md) · [Configuration](Configuration.md)

## Read the logs

For a foreground run, stop the affected application first, then start it with debug
logging from a terminal in Niri:

```sh
RUST_LOG=debug ghost-shell-bar
```

For the Home Manager user service:

```sh
systemctl --user status ghost-shell-bar.service
journalctl --user -u ghost-shell-bar.service -b
```

## No bar appears

- Check for `Configuration:`. Loading errors cause all settings to fall
  back to defaults, which show a bar on each connected output.
- Check `config.toml`; it takes precedence over the older application-specific files.
- Compare `niri msg outputs` with your `[bar."OUTPUT"]` table keys. Setting only
  `output = "DP-1"` inside `[bar.main]` will not select `DP-1`.

## Changes have no effect

Restart the affected application after editing its configuration file. Unknown fields can be ignored;
compare the spelling and section against the [configuration reference](Configuration.md).
Widget ordering and most widget-specific settings are not configurable yet.

## CLI cannot reach an application

Run the application and CLI in the same user session with `XDG_RUNTIME_DIR` set.
Sockets are named `ghost-shell-bar` and `ghost-shell-lockscreen` inside that directory. Start the target application's
service; the CLI does not start it for you. Check the service's `WAYLAND_DISPLAY`
environment condition and that `graphical-session.target` is active. Niri-dependent
features also need `NIRI_SOCKET` and desktop services need the session D-Bus environment.

Launcher and finder have no IPC sockets. Invoke `ghost-shell-launcher` or
`ghost-shell-finder` directly. Stop any old launcher/finder services and remove
their autostart entries when upgrading; those services must not restart the apps
after dismissal.

## Lockscreen service stop is waiting

Stop/restart while locked intentionally waits for authentication. Unlock normally
to complete the operation; do not force-kill the lock client. See [lockscreen](Lockscreen.md).

## Wallpaper does not appear

Check the file exists, is readable, and is PNG, JPEG, or GIF. Use an absolute
path without `~` or shell variables. Remove `wallpaper.path` to use the solid
background. Invalid image paths are logged and leave the configured solid fallback
visible.

## Feature-specific issues

- No blur: compositor support is required; opacity alone does not enable blur.
- Terminal application fails to launch: the launcher currently requires Ghostty.
- Finder does not open files: opening results is not implemented yet.
- Lock authentication fails: check your system's PAM policy for `ghost-shell` and
  the `USER` environment variable; see [lockscreen](Lockscreen.md).
- Audio unavailable: check the session's PipeWire service and the widget's status
  message; see [audio](widgets/Audio.md).
- Battery value does not change: periodic refresh is not implemented yet.

Consult each [widget page](Widgets.md) for its implementation status before
assuming an absent feature is a configuration problem.
