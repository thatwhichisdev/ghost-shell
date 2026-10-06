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

Run the daemon from a terminal in your Niri session:

```sh
ghost-shell-daemon
```

In another terminal, open the launcher:

```sh
ghost-shell msg launcher toggle
```

See [commands](Commands.md) for the finder and session lock actions.

## Integrate with Niri

If you are not using the Home Manager service, add an autostart command to your
Niri configuration:

```kdl
spawn-at-startup "ghost-shell-daemon"
```

Add bindings inside your existing `binds` block, choosing keys that do not
conflict with your configuration:

```kdl
binds {
    Mod+D { spawn "ghost-shell" "msg" "launcher" "toggle"; }
    Mod+E { spawn "ghost-shell" "msg" "finder" "toggle"; }
    Mod+Shift+L { spawn "ghost-shell" "msg" "session" "lock"; }
}
```

Complete [lockscreen authentication setup](Lockscreen.md) before using the lock
binding. These are Niri settings, not Ghost Shell TOML settings. Refer to
[Niri's configuration guide](https://github.com/niri-wm/niri/wiki/Configuration:-Introduction)
for compositor configuration.

Restart the daemon after editing Ghost Shell's configuration. If using the
Home Manager service, run `systemctl --user restart ghost-shell.service`.
