# Installation

[Wiki](README.md) · Next: [first session](Getting-Started.md)

Ghost Shell is in early alpha. This guide covers the source and Nix installation
paths provided by the repository.

## Requirements

Run Ghost Shell in a Linux Niri Wayland session. Applications need the session's
Wayland connection, `NIRI_SOCKET`, `XDG_RUNTIME_DIR`, and session D-Bus connection.
Audio controls use PipeWire; terminal applications currently use Ghostty.
Lockscreen authentication uses the PAM service `ghost-shell`; see
[lockscreen setup](Lockscreen.md).

## Build from source

Follow the [development prerequisites](../CONTRIBUTING.md#prerequisites), then
build all six executables from the repository root:

```sh
cargo build --workspace --release
```

Install the control CLI (`ghost-shell`) and the four application executables:

```sh
for program in ghost-shell ghost-shell-bar ghost-shell-launcher ghost-shell-finder ghost-shell-lockscreen; do
    install -Dm755 "target/release/$program" "$HOME/.local/bin/$program"
done
```

Applications run independently; there is no supervisor binary. Use Home Manager
services or start the applications individually in your session. The additional
`ghost-shell-greeter` executable must be installed system-wide for boot login;
use the [NixOS greeter module](Greeter.md#nixos) for that setup.

Ensure `$HOME/.local/bin` is on the `PATH` available to Niri. Native runtime
libraries must also be available; the development guide lists them.

## Nix package

With flakes enabled, build from the checkout:

```sh
nix build .#ghost-shell
```

All six executables are in `result/bin/`. You can run them by that path, or install
the package into your user profile:

```sh
nix profile add .#ghost-shell
```

The flake exposes packages for `x86_64-linux` and `aarch64-linux`.

## Home Manager

The flake exports `homeManagerModules.ghost-shell` (also available as `default`).
Add Ghost Shell as an input to your own flake:

```nix
inputs.ghost-shell.url = "github:thatwhichisdev/ghost-shell";
```

Pass `inputs` to your Home Manager modules, then import and configure the module:

```nix
{ inputs, ... }:
{
  imports = [ inputs.ghost-shell.homeManagerModules.ghost-shell ];

  programs.ghost-shell = {
    enable = true;
    systemd.enable = true;
    settings = {
      bar."eDP-1" = {
        output = "eDP-1";
        primary = true;
      };
      clock.format = "%H:%M";
    };
  };
}
```

Replace `eDP-1` with your output name. `programs.ghost-shell.settings` generates
`$XDG_CONFIG_HOME/ghost-shell/config.toml`. Define shared `general`, `theme` and
`wallpaper` settings once alongside `bar`, `clock`, `launcher` and `finder`.
Set `applications.<name>.enable = false` to omit that application's service;
the shared file remains. Only `bar` and `lockscreen` have service enable options.
Launcher and finder are installed with the package and launched directly from
Niri bindings. Remove their old `applications.launcher.enable` and
`applications.finder.enable` options.

With `systemd.enable = true`, Home Manager creates `ghost-shell-bar.service` and
`ghost-shell-lockscreen.service`. They are bound to `graphical-session.target`
and require `WAYLAND_DISPLAY` in the user service manager environment. Ensure
`NIRI_SOCKET` and the session D-Bus environment are also available. Use either
these services or Niri autostart, avoiding duplicate instances.

Normal applications restart on failure independently. The lockscreen does not
automatically restart after a crash. Its stop/restart waits for authentication
while locked, with no force-kill timeout. The module does not install a PAM policy.

Migration: stop the old launcher and finder services before switching to direct
launches (`systemctl --user stop ghost-shell-launcher ghost-shell-finder`), remove
their Niri autostart entries, and replace their IPC shortcuts with direct executable
bindings. A Home Manager switch removes their old generated units.

Remove old references to `ghost-shell.service` and
`ghost-shell-daemon`. Move any `applications.<name>.settings` sections into the shared `settings`
option as described in [configuration](Configuration.md#migrating-to-configtoml).

Implementation: [flake](../../flake.nix), [package](../../nix/package.nix), and
[Home Manager module](../../nix/module.nix).
