# Ghost Shell wiki

Ghost Shell is an early-alpha desktop shell for the Niri Wayland compositor,
built with iced. It provides a bar, launcher, finder, desktop wallpapers,
lockscreen, and optional boot greeter, while delegating window management to
Niri.

Start with Installation to set up Ghost Shell on NixOS using flakes and Home
Manager. Configuration describes supported settings and current limitations.
Development defines the principles for designing and implementing Ghost Shell.
The [project README](../../README.md) contains screenshots and the roadmap.

## Getting Started

- [Installation](getting-started/Installation.md): NixOS flakes, Home Manager,
  first session, Niri bindings, commands, and troubleshooting.

## Configuration

- [Introduction](configuration/Introduction.md): file discovery, defaults,
  shared settings, themes, wallpapers, and configuration migration.
- [Bar](configuration/Bar.md): outputs, appearance, fixed layout, and widget
  status.
- [Styling](configuration/Styling.md): live bar configuration, preview/reset,
  palette editing, and exporting Nix settings.
- [Menu](configuration/Menu.md): shutdown, reboot, lock, and live styling.
- [Tray](configuration/Tray.md): session integration and application menus.
- [Audio](configuration/Audio.md): PipeWire devices, active-device selection,
  and volume controls.
- [Clock](configuration/Clock.md): time format and update frequency.
- [Workspaces](configuration/Workspaces.md): Niri workspace indicators.
- [Launcher](configuration/Launcher.md): application discovery and appearance.
- [Finder](configuration/Finder.md): file search and appearance.
- [Lockscreen](configuration/Lockscreen.md): PAM setup, wallpaper, and locking.
- [Greeter](configuration/Greeter.md): greetd, NixOS setup, and login
  configuration.

## Development

- [Design Principles and
  Architecture](development/Design-Principles-and-Architecture.md): crate
  responsibilities, runtime ownership, state flow, and implementation rules.
- [UI/UX](development/UI-UX.md): visual identity, interaction, focus, feedback,
  and interface validation.

For toolchain setup and development commands, see
[Contributing](../CONTRIBUTING.md). [AGENTS.md](../../AGENTS.md) links these
design guides and supplies actionable instructions for coding agents. This wiki
is plain Markdown browsable in the repository; no publishing pipeline is
required.
