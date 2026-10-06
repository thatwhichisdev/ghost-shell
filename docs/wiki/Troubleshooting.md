# Troubleshooting

[Wiki](README.md) · [Configuration](Configuration.md)

## Read the logs

For a foreground run, stop the existing daemon first, then start it with debug
logging from a terminal in Niri:

```sh
RUST_LOG=debug ghost-shell-daemon
```

For the Home Manager user service:

```sh
systemctl --user status ghost-shell.service
journalctl --user -u ghost-shell.service -b
```

## No bar appears

- Check for `Failed to load config`. Loading errors cause all settings to fall
  back to defaults, which contain no bars.
- Compare `niri msg outputs` with your `[bar."OUTPUT"]` table keys. Setting only
  `output = "DP-1"` inside `[bar.main]` will not select `DP-1`.
- Look for `Failed to initialize bar power widget`. Currently the whole bar
  manager stops if no suitable battery can be found. There is no configuration
  switch to disable just the power widget.

## Changes have no effect

Restart the daemon after configuration edits. Unknown fields can be ignored;
compare the spelling and section against the [configuration reference](Configuration.md).
Widget ordering and most widget-specific settings are not configurable yet.

## CLI cannot reach the daemon

Run the daemon and CLI in the same user session with `XDG_RUNTIME_DIR` set.
The socket is `$XDG_RUNTIME_DIR/ghost-shell-daemon`. Also ensure Niri's
`NIRI_SOCKET` and the Wayland/session D-Bus environment are available to the
daemon. If a service does not start, check its `NIRI_SOCKET` environment condition
and that `graphical-session.target` is active.

## Wallpaper prevents startup

Check the file exists, is readable, and is PNG, JPEG, or GIF. Use an absolute
path without `~` or shell variables. Remove `wallpaper.path` to use the solid
background; invalid image paths currently fail startup.

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
