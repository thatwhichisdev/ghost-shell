# First session

[Wiki](README.md) · [Installation](Installation.md) · [Configuration](Configuration.md)

## Create a configuration

Inside Niri, inspect output names:

```sh
niri msg outputs
```

Create `${XDG_CONFIG_HOME:-$HOME/.config}/ghost-shell/config.toml` and replace
`eDP-1` in this example with an actual output name:

```toml
[general]
font_family = "monospace"
font_size = 13.0

[bar."eDP-1"]
output = "eDP-1"
primary = true
height = 27.0
exclusive_zone = 27.0
appearance = "themed"

[clock]
format = "%H:%M"
```

The bar table name must match the output; see [bar configuration](Bar.md).
Currently, a failure to find a suitable battery prevents bar initialization,
including on desktops without batteries. See [power](widgets/Power.md).

## Start and control the shell

For a foreground run, start the two daemons in separate terminals in Niri:

```sh
ghost-shell-bar
ghost-shell-lockscreen
```

Launch the launcher on demand from another terminal:

```sh
ghost-shell-launcher
```

See [commands](Commands.md) for the finder and session lock actions.

## Integrate with Niri

Prefer the [Home Manager services](Installation.md#home-manager). If you do not
use Home Manager, Niri can start the binaries independently:

```kdl
spawn-at-startup "ghost-shell-bar"
spawn-at-startup "ghost-shell-lockscreen"
```

Do not enable both startup mechanisms for the same application.

Add bindings inside your existing `binds` block, choosing keys that do not
conflict with your configuration:

```kdl
binds {
    Mod+D { spawn "ghost-shell-launcher"; }
    Mod+E { spawn "ghost-shell-finder"; }
    Mod+Shift+L { spawn "ghost-shell" "msg" "session" "lock"; }
}
```

Complete [lockscreen authentication setup](Lockscreen.md) before using the lock
binding. These are Niri settings, not Ghost Shell TOML settings. Refer to
[Niri's configuration guide](https://github.com/niri-wm/niri/wiki/Configuration:-Introduction)
for compositor configuration.

Restart the affected application after configuration changes, for example
`systemctl --user restart ghost-shell-bar.service`. Lock service stop/restart
waits for authentication if the session is locked.
