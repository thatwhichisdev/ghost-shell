# Installation

[Wiki](../README.md#getting-started) · Next:
[Configuration](../configuration/Introduction.md)

Ghost Shell is an early-alpha desktop shell for the Niri Wayland compositor.
This guide installs its desktop applications on NixOS using flakes and Home
Manager, then connects them to your Niri session. The package supports
`x86_64-linux` and `aarch64-linux`.

## Requirements

Use a NixOS configuration with flakes enabled and a working Niri session.
Applications need the session's Wayland connection, `NIRI_SOCKET`,
`XDG_RUNTIME_DIR`, and D-Bus environment. Audio controls need PipeWire; terminal
applications currently launch through Ghostty.

The examples extend an existing NixOS configuration. Replace `hostname` and
`alice` with your host and user names. Keep your existing hardware,
user-account, bootloader, and state-version settings.

## Add the flake inputs

Add Ghost Shell and Home Manager to your system flake. If you already use Home
Manager, keep its setup and add only the Ghost Shell pieces. Use a Home Manager
branch compatible with nixpkgs; this example uses unstable for both.

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    ghost-shell.url = "github:thatwhichisdev/ghost-shell";
  };

  outputs = inputs@{ nixpkgs, home-manager, ... }: {
    nixosConfigurations.hostname = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      specialArgs = { inherit inputs; };
      modules = [
        ./configuration.nix
        home-manager.nixosModules.home-manager
        {
          home-manager.useGlobalPkgs = true;
          home-manager.useUserPackages = true;
          home-manager.extraSpecialArgs = { inherit inputs; };
          home-manager.users.alice = import ./home.nix;
        }
      ];
    };
  };
}
```

`specialArgs` passes `inputs` to NixOS modules; Home Manager needs its own
`extraSpecialArgs`. Ghost's desktop module is a Home Manager module. The
separate NixOS module configures only the optional greeter.

## Configure NixOS and authentication

Enable Niri in your existing `configuration.nix`:

```nix
programs.niri.enable = true;
```

The lockscreen authenticates through a PAM service named `ghost-shell`. Provide
that service through your NixOS `security.pam.services` policy before enabling
locking. Neither the package nor Home Manager installs the policy. See
[Lockscreen](../configuration/Lockscreen.md#authentication-setup) for
requirements and diagnostics. You can use the other applications while setting
up authentication.

## Home Manager

Add the following to `home.nix`, retaining your existing settings, including
`home.stateVersion`:

```nix
{ inputs, ... }:
{
  imports = [ inputs.ghost-shell.homeManagerModules.ghost-shell ];

  programs.ghost-shell = {
    enable = true;
    systemd.enable = true;
    applications.lockscreen.enable = false;
    settings = {
      general = {
        font_family = "monospace";
        font_size = 13.0;
      };
      clock.format = "%H:%M";
    };
  };
}
```

This installs all six executables and generates
`$XDG_CONFIG_HOME/ghost-shell/config.toml`, normally
`~/.config/ghost-shell/config.toml`. With no `bar` entries, a bar opens on every
connected output. Use `niri msg outputs` and the [Bar](../configuration/Bar.md)
guide to select outputs. Shared fonts, themes, and wallpapers are covered in
[Introduction](../configuration/Introduction.md).

`systemd.enable` defaults to `false`. Enabling it creates independent user
services for the enabled daemons. `applications.bar.enable` and
`applications.lockscreen.enable` both default to `true`; the example disables
locking until PAM is ready. Then set `applications.lockscreen.enable = true;`
and test authentication. Launcher and finder run on demand and have no services.

The module is also exported as `homeManagerModules.default`. With standalone
Home Manager, import the same module and pass `inputs` through its
`extraSpecialArgs`; Niri and PAM still belong in the NixOS configuration.

## Apply and start a session

From the directory containing your system flake:

```sh
sudo nixos-rebuild switch --flake .#hostname
```

Log into Niri. The services are bound to `graphical-session.target` and require
`WAYLAND_DISPLAY` in the user service manager environment. Niri's socket and the
session D-Bus environment must also be available. Check the bar with:

```sh
systemctl --user status ghost-shell-bar.service
journalctl --user -u ghost-shell-bar.service -b
```

The bar restarts on failure. The lockscreen does not automatically restart after
a crash; stopping or restarting it while locked waits for authentication.
Rebuild after changing Home Manager settings. Changes to the generated config
reload the bar live and restart the lockscreen service. Launcher and finder
read the file on each invocation.

## Integrate with Niri

Add these entries to your existing `binds` block, choosing available keys:

```kdl
binds {
    Mod+D { spawn "ghost-shell-launcher"; }
    Mod+E { spawn "ghost-shell-finder"; }
    Mod+Shift+L { spawn "ghost-shell" "msg" "session" "lock"; }
}
```

Use the lock binding after enabling the service and testing PAM setup. These are
compositor settings, not Ghost Shell TOML settings.

Without Home Manager services, Niri can start the daemons instead:

```kdl
spawn-at-startup "ghost-shell-bar"
spawn-at-startup "ghost-shell-lockscreen"
```

Use one startup mechanism per daemon. For a foreground run, start each daemon in
a separate terminal within Niri. Launcher and finder run on demand.

## Optional boot greeter

Follow [Greeter](../configuration/Greeter.md#nixos) to use Ghost Shell before
login. It uses a separate NixOS module, greetd authentication, and system-owned
configuration. Installing the desktop applications does not enable it.

## Updating an older installation

Stop old launcher and finder services before switching to direct launches:

```sh
systemctl --user stop ghost-shell-launcher ghost-shell-finder
```

Remove their service options and Niri autostart entries; replace old IPC
bindings with the executable names above. A Home Manager switch removes the old
units. Remove references to `ghost-shell.service` and `ghost-shell-daemon`;
there is no supervisor executable. For per-application settings, follow
[configuration
migration](../configuration/Introduction.md#migrating-to-configtoml).

## Other installation methods

From a checkout, `nix build .#ghost-shell` produces all six executables in
`result/bin/`; `nix profile add .#ghost-shell` installs them in a user profile.
These commands do not configure services. For source builds, see
[Contributing](../../CONTRIBUTING.md).

Implementation: [flake](../../../flake.nix),
[package](../../../nix/package.nix), and [Home Manager
module](../../../nix/module.nix).

## Commands and IPC

`ghost-shell` controls the running bar and lockscreen daemons. Launcher and
finder are regular applications launched directly, without IPC servers.

  | Command                                         | Action                                                      |
  | ----------------------------------------------- | ----------------------------------------------------------- |
  | `ghost-shell-launcher`                          | Open the launcher; exit after a successful launch or Escape |
  | `ghost-shell-finder`                            | Open the finder; exit on Escape or surface closure          |
  | `ghost-shell msg bar open` / `close` / `toggle` | Show or hide bars and their popups; wallpaper remains       |
  | `ghost-shell msg session lock`                  | Lock the session; acknowledge compositor acceptance         |
  | `ghost-shell msg shutdown bar` / `lockscreen`   | Stop the daemon; an active lock waits for authentication    |

Repeated launcher/finder invocations exit when another instance already holds
that application's process lock. They do not toggle or control the existing
window. Replace old `msg launcher` and `msg finder` bindings with the executable
names.

Inspect CLI syntax with:

```sh
ghost-shell --help
ghost-shell msg --help
```

The CLI connects directly to `$XDG_RUNTIME_DIR/ghost-shell-{bar,lockscreen}`
according to the command. `XDG_RUNTIME_DIR` must be set and shared with the
target. Daemons are not started implicitly; a missing socket produces a
connection error. There are no reload or theme-switch subcommands.

UI `Handled` replies acknowledge accepted commands, not a completed render. Lock
replies wait for compositor acceptance. Closing launcher/finder surfaces
terminates their processes. `ghost-shell msg shutdown lockscreen` acknowledges
the stop request immediately and defers exit until unlock when locked. Stopping
one application does not stop any other application.

For global shortcuts, bind these commands in
[Niri](Installation.md#integrate-with-niri). For lock prerequisites, see
[lockscreen](../configuration/Lockscreen.md).

Implementation:
[CLI](../../../crates/ghost-shell-applications/ghost-shell-cli/src/main.rs) and
[IPC
protocol](../../../crates/ghost-shell-adapters/ghost-shell-ipc/src/protocol.rs).

## Troubleshooting

### Read the logs

For a foreground run, stop the affected application first, then start it with
debug logging from a terminal in Niri:

```sh
RUST_LOG=debug ghost-shell-bar
```

For the Home Manager user service:

```sh
systemctl --user status ghost-shell-bar.service
journalctl --user -u ghost-shell-bar.service -b
```

### No bar appears

- Check for `Configuration:`. Loading errors cause all settings to fall back to
  defaults, which show a bar on each connected output.
- Check `config.toml`; it takes precedence over the older application-specific
  files.
- Compare `niri msg outputs` with your `[bar.outputs."OUTPUT"]` table keys.
  Geometry and decoration belong in the global `[bar]` settings.

### Changes have no effect

Restart the affected application after editing its configuration file. Unknown
fields can be ignored; compare the spelling and section against the
[configuration reference](../configuration/Introduction.md). Widget ordering and
most widget-specific settings are not configurable yet.

### CLI cannot reach an application

Run the application and CLI in the same user session with `XDG_RUNTIME_DIR` set.
Sockets are named `ghost-shell-bar` and `ghost-shell-lockscreen` inside that
directory. Start the target application's service; the CLI does not start it for
you. Check the service's `WAYLAND_DISPLAY` environment condition and that
`graphical-session.target` is active. Niri-dependent features also need
`NIRI_SOCKET` and desktop services need the session D-Bus environment.

Launcher and finder have no IPC sockets. Invoke `ghost-shell-launcher` or
`ghost-shell-finder` directly. Stop any old launcher/finder services and remove
their autostart entries when upgrading; those services must not restart the apps
after dismissal.

### Lockscreen service stop is waiting

Stop/restart while locked intentionally waits for authentication. Unlock
normally to complete the operation; do not force-kill the lock client. See
[lockscreen](../configuration/Lockscreen.md).

### Wallpaper does not appear

Check the file exists, is readable, and is PNG, JPEG, or GIF. Use an absolute
path without `~` or shell variables. Remove `wallpaper.path` to use the solid
background. Invalid image paths are logged and leave the configured solid
fallback visible.

### Feature-specific issues

- No blur: compositor support is required; opacity alone does not enable blur.
- Terminal application fails to launch: the launcher currently requires Ghostty.
- Finder does not open files: opening results is not implemented yet.
- Lock authentication fails: check your system's PAM policy for `ghost-shell`
  and the `USER` environment variable; see
  [lockscreen](../configuration/Lockscreen.md).
- Audio unavailable: check the session's PipeWire service and the widget's
  status message; see [audio](../configuration/Audio.md).
- Battery value does not change: periodic refresh is not implemented yet.

Consult each [widget page](../configuration/Bar.md#widgets) for its
implementation status before assuming an absent feature is a configuration
problem.
