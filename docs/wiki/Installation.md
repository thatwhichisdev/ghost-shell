# Installation

[Wiki](README.md) · Next: [first session](Getting-Started.md)

Ghost Shell is in early alpha. This guide covers the source and Nix installation
paths provided by the repository.

## Requirements

Run Ghost Shell in a Linux Niri Wayland session. The daemon needs the session's
Wayland connection, `NIRI_SOCKET`, `XDG_RUNTIME_DIR`, and session D-Bus connection.
Audio controls use PipeWire; terminal applications currently use Ghostty.
Lockscreen authentication uses the PAM service `ghost-shell`; see
[lockscreen setup](Lockscreen.md).

## Build from source

Follow the [development prerequisites](../CONTRIBUTING.md#prerequisites), then
build both executables from the repository root:

```sh
cargo build --release -p ghost-shell-daemon -p ghost-shell-cli
```

The resulting programs are `target/release/ghost-shell-daemon` (the desktop shell)
and `target/release/ghost-shell` (its control CLI). Install them into a directory
on your session's `PATH`, for example:

```sh
install -Dm755 target/release/ghost-shell-daemon "$HOME/.local/bin/ghost-shell-daemon"
install -Dm755 target/release/ghost-shell "$HOME/.local/bin/ghost-shell"
```

Ensure `$HOME/.local/bin` is on the `PATH` available to Niri. Native runtime
libraries must also be available; the development guide lists them.

## Nix package

With flakes enabled, build from the checkout:

```sh
nix build .#ghost-shell
```

Both executables are in `result/bin/`. You can run them by that path, or install
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

Replace `eDP-1` with your output name. `settings` generates
`$XDG_CONFIG_HOME/ghost-shell/config.toml`. The optional systemd user service is
bound to `graphical-session.target` and requires `NIRI_SOCKET` in the user service
manager's environment. Use either this service or Niri autostart to launch the
daemon, avoiding duplicate instances. The module does not install a PAM policy.

Implementation: [flake](../../flake.nix), [package](../../nix/package.nix), and
[Home Manager module](../../nix/module.nix).
