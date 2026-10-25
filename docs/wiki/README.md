# Ghost Shell wiki

Ghost Shell is an early-alpha desktop shell for the Niri Wayland compositor,
built with iced. It provides a bar, an application launcher, a file finder,
wallpapers, and a lockscreen, while delegating window management to Niri.

These guides describe the current implementation. The
[project README](../../README.md) contains screenshots and the roadmap; unfinished
features are called out in the relevant guide.

## Getting started

- [Installation](Installation.md): build or install the executables.
- [First session](Getting-Started.md): configure outputs, start the daemons, and
  connect Niri key bindings.
- [Commands](Commands.md): control the running shell.
- [Troubleshooting](Troubleshooting.md): diagnose setup and alpha limitations.

## Configuration and features

- [Configuration](Configuration.md): file location, defaults, and general options.
- [Bar and outputs](Bar.md): monitor selection, dimensions, and appearance.
- [Widgets](Widgets.md): behavior, setup, and supported settings for each widget.
- [Theming](Theming.md): fonts, appearance mode, and Base16 palettes.
- [Launcher](Launcher.md): find and launch applications.
- [Finder](Finder.md): search files and directories.
- [Wallpapers](Wallpapers.md): static images, GIFs, and background colors.
- [Lockscreen](Lockscreen.md): locking and authentication setup.
- [Greeter](Greeter.md): boot login through greetd and NixOS configuration.

## Understanding and developing Ghost Shell

- [Architecture](Architecture.md): surface coordination, feature state,
  and backend boundaries.
- [Contributing guide](../CONTRIBUTING.md): toolchain, builds, checks, and writing docs.
- [Agent guide](../../AGENTS.md): coding conventions and iced patterns.

The organization takes inspiration from [Niri's wiki](https://github.com/niri-wm/niri/wiki):
a getting-started path, focused configuration pages, and separate development
material. This directory is the repository's documentation source; it is not
currently synchronized to a separate GitHub Wiki.
