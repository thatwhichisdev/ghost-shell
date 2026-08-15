# Preview

![preview](assets/preview.png)

# Overview

Ghost Shell is a desktop shell built exclusively for the Niri Wayland
compositor. The project's name is inspired by the anime Ghost in the Shell.

I simply want to build minimalist looking and resource efficient tool that I
will enjoy using on the daily basis. The project is currently in a very early
stage of development, so it is not yet ready for general use.

The current scope before 0.1.0 release is to implement following:

- [ ] Bar
  - [ ] Widget: System Menu
  - [x] Widget: Niri Workspaces
  - [x] Widget: Focused Window
  - [ ] Widget: Tray
  - [ ] Widget: Notifications
  - [ ] Widget: Camera
  - [ ] Widget: Audio Control (Speakers/Microfone)
  - [ ] Widget: Bluetooth Control
  - [ ] Widget: Network Control (Wifi/Ethernet)
  - [ ] Widget: Power Control (Battery/CPU modes)
  - [x] Widget: Clock
  - [ ] Widget: Screenshot & Screen recording
  - [ ] Widget: Theme Polarity Changer
  - [ ] Widget: Weather
- [x] Launcher
- [x] Finder
- [ ] Lock Screen
- [ ] Clipboard
- [ ] Wallpapers
- [ ] Theming

# Getting Started

## Prerequisities

Development and packaging is currently intended for NixOS systems. There are no
plans to support other systems at this moment.

## Installation

Clone the repository:

```shell
git clone https://github.com/thatwhichisdev/ghost-shell
```

## Developing

To enter a development environment with all the necessary tools, run:

```shell
nix develop
```

## Building

```
cargo build  
```

## Running

```
cargo run   
```

## Configuration

Ghost Shell discovers its configuration according to the XDG Base Directory
Specification, for example `~/.config/ghost-shell/config.toml`.

```toml
[general]
font_family = "BerkeleyMono Nerd Font Mono"
font_size = 13
fg = 0xffffffff
bg = 0x00000000

[bar."eDP-1"]
output = "eDP-1"
height = 27.0
exclusive_zone = 27.0

[bar."DP-1"]
primary = true
output = "DP-1"
height = 27.0
exclusive_zone = 27.0

[clock]
format = "%H:%M"
```

# Acknowledgments

Ghost Shell would not be possible without the excellent work of the projects it
builds upon:

- Niri — an excellent scrollable Wayland compositor with a thoughtful design and
  a surprisingly rich set of built-in capabilities, including IPC, application
  spawning, workspace management, and much more.
- GPUI — Zed's GPU-accelerated UI framework. Fast, expressive, and genuinely
  enjoyable to build native interfaces with.
- GPUI Component — a comprehensive component library for GPUI that provides most
  of the building blocks needed to get a polished interface running quickly.
- awww — a fast and beautifully implemented animated wallpaper daemon that keeps
  Ghost's desktop background considerably less boring.

# Licensing

The code in this project is licensed under the MIT License. Check the
[LICENSE](LICENSE.md) file for further details.
