# Greeter

[Wiki](README.md) · [Lockscreen](Lockscreen.md) · [Installation](Installation.md)

`ghost-shell-greeter` is a graphical login interface for greetd. It displays a
wallpaper and clock on every connected output. Enter your username and press
**Log in** (or Enter); greetd then supplies the password prompt and any additional
questions required by your PAM policy. Secret responses are masked. Informational
messages are displayed and acknowledged automatically. **Change user** cancels
an active prompt before returning to username entry.

The greeter starts a new session. To lock and return to an existing desktop, keep
using `ghost-shell msg session lock` and the separate lockscreen user service.
The greeter does not use the lockscreen's PAM service or read the desktop's
shared `config.toml`.

## NixOS

Add Ghost's flake input to your system flake, pass `inputs` through your NixOS
`specialArgs`, and import [the example module](../../nix/examples/greeter.nix), or
include the following in your NixOS configuration:

```nix
{ inputs, config, ... }:
{
  imports = [ inputs.ghost-shell.nixosModules.greeter ];
  programs.niri.enable = true;

  services.ghost-shell-greeter = {
    enable = true;
    settings = {
      session.command = [ "${config.programs.niri.package}/bin/niri-session" ];
      wallpaper.path = "${./wallpaper.jpg}";
      theme.mode = "dark";
    };
  };
}
```

Place `wallpaper.jpg` beside your configuration, or omit `wallpaper.path` for a
solid background. Interpolating the path copies it into the Nix store so it is
available before login. GIF, PNG and JPEG use the existing wallpaper renderer.
Avoid paths inside private or encrypted home directories. The Nix store is public;
do not put credentials or private assets in these settings.

Replace your existing `services.greetd.settings.default_session` definition for
tuigreet; do not keep both command definitions. The module enables greetd and
configures a dedicated minimal Sway compositor under the `greeter` account. It
has no terminal-launch bindings or user configuration includes. When Ghost exits,
a wrapper exits Sway as well, allowing greetd to start the authenticated session.
Logout returns to the greeter through greetd. This does not start the Ghost bar,
launcher, finder or lockscreen in the login environment.

The generated application configuration is exposed at
`/etc/ghost-shell/greeter.toml`. The module passes its immutable store path to the
application. NixOS supplies greetd's PAM policy. The separate lockscreen still
requires the `ghost-shell` PAM policy described in its guide.

The default session command is nixpkgs' `niri-session`; the example explicitly uses
the package selected by `programs.niri.package`. Environment defaults identify a
Wayland/Niri session. Other sessions require overriding both `session.command` and
`session.environment`. There is no interactive session selector or automatic login.

## Configuration outside the module

The application defaults to `/etc/ghost-shell/greeter.toml`, or accepts
`--config /absolute/path/greeter.toml`. The file is required, and invalid
configuration prevents startup. It uses the existing `[general]`, `[theme]` and
`[wallpaper]` schemas plus `[session]`:

```toml
[general]
font_family = "monospace"
font_size = 13.0

[wallpaper]
path = "/etc/ghost-shell/wallpaper.jpg"
bg = 658448

[theme]
mode = "dark"

[session]
command = ["/run/current-system/sw/bin/niri-session"]
environment = ["XDG_SESSION_TYPE=wayland", "XDG_CURRENT_DESKTOP=niri", "XDG_SESSION_DESKTOP=niri"]
```

The session command must be configured explicitly. Start the greeter through
greetd inside a layer-shell-capable compositor; launching it from an ordinary
terminal does not supply `GREETD_SOCK`. Transport failures show an error and a
**Restart greeter** action. In the supplied NixOS setup this ends the greeter
compositor, and greetd recreates the login session. Failed authentication cancels
the transaction and permits another attempt. Requests are serialized; controls
wait while greetd processes an authentication response.

## Implementation and validation

The application independently adapts the lockscreen's view, clock, theme and
wallpaper loading. It owns its Waycrate runtime and layer surfaces; it neither
locks a session nor exposes the user-session command socket. Its external control
connection is greetd's socket. The adapter serializes exchanges and invalidates
the connection after interrupted or failed transport operations. Session start is
never automatically retried. The application exits only after greetd acknowledges
session start, or when the user explicitly restarts after a fatal error.

No tests were added. Before deployment, validate login, authentication failure,
additional PAM prompts, logout, output hotplug, keyboard focus/scaling, and
compositor handoff on the target machine. Compilation cannot establish these.

Sources: [greetd IPC specification](https://man.archlinux.org/man/greetd-ipc.7.en),
[graphical greeter lifecycle](https://man.archlinux.org/man/gtkgreet.1.en),
[tuigreet IPC reference](https://github.com/tuigreet/tuigreet/blob/master/crates/tuigreet/src/ipc.rs),
[Ghost adapter](../../crates/ghost-shell-adapters/ghost-shell-greetd/src/greetd.rs),
[Ghost application](../../crates/ghost-shell-applications/ghost-shell-greeter/src/app.rs).
