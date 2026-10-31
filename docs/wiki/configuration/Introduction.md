# Introduction

[Wiki](../README.md#configuration) ·
[Installation](../getting-started/Installation.md)

Ghost Shell uses TOML for desktop configuration. Start with the shared settings
below, then use the application and widget pages for supported options. Niri
bindings and system services are configured separately.

## File discovery and loading

Desktop applications independently read
`$XDG_CONFIG_HOME/ghost-shell/config.toml`, normally
`~/.config/ghost-shell/config.toml`. Each deserializes only its relevant
sections; there is no shared runtime or dependency on another application. There
is no desktop command-line config override or environment-variable option for
individual settings. The bar reloads valid file changes live. Launcher and finder
read settings on invocation; restart other applications after edits.

  | Application | Sections read                                                      |
  | ----------- | ------------------------------------------------------------------ |
  | Bar         | `[general]`, `[theme]`, `[bar]`, `[clock]`, `[wallpaper]` |
  | Launcher    | `[general]`, `[theme]`, `[launcher]`                               |
  | Finder      | `[general]`, `[theme]`, `[finder]`                                 |
  | Lockscreen  | `[general]`, `[theme]`, `[wallpaper]`                              |

Define `[general]`, `[theme.dark]` and `[theme.light]` once for all desktop
applications. `[wallpaper]` supplies the desktop and lock background. There are
no application-specific theme overrides. The boot [greeter](Greeter.md) retains
its own system-owned `/etc/ghost-shell/greeter.toml`, accessible before login.

Omitted sections and fields use defaults, except that an explicitly supplied
Base16 palette requires all sixteen colors. Invalid configuration logs
`Configuration:` and falls back to that application's defaults at startup.
During bar reloads, invalid or removed files preserve the last working settings
and show an error in the styling panel. With no
`[bar.outputs.*]` entries, the bar appears on every connected output. Unknown sections
and fields are generally ignored (bar settings reject unknown fields), so a type error in an unrelated application's section
does not prevent another application from reading its own settings. TOML syntax
errors still affect the whole file.

## Migrating to config.toml

Combine application sections into `config.toml`, choosing one shared theme,
general configuration and wallpaper. Bar settings now use global `[bar]`,
`[bar.decoration]`, `[bar.widgets]` and `[bar.outputs."OUTPUT"]` tables; see
[bar migration](Bar.md#migration). Move `general.theme` to `theme.icons`.

For compatibility, when `config.toml` is absent, each application falls back to
its previous `bar.toml`, `launcher.toml`, `finder.toml` or `lockscreen.toml`.
Files are never merged: an existing `config.toml`, even an empty or invalid one,
takes precedence. If neither file exists, defaults apply.

Home Manager generates the shared file through `programs.ghost-shell.settings`.
Move sections out of `applications.<name>.settings` into that option; the
removed options report a migration error. `applications.<name>.enable` still
controls the bar and lockscreen services. A shared configuration change is reloaded by the bar without a service restart.
Lockscreen still restarts and waits for authentication while locked. See
[installation](../getting-started/Installation.md#home-manager).

Launcher and finder let Niri select the active output. They do not read the
bar's primary-output setting. There are no dedicated sections for other widgets.
See [widgets](Bar.md#widgets).

## General settings

```toml
[general]
font_family = "monospace"
font_size = 13.0
```

  | Option        | Type   | Default       | Behavior                                                             |
  | ------------- | ------ | ------------- | -------------------------------------------------------------------- |
  | `font_family` | String | `"monospace"` | Sets the default UI font family; install your chosen font separately |
  | `font_size`   | Number | `13.0`        | Sets the default UI text size                                        |

## Surface opacity

Bars use global `bar.decoration.opacity` (default `1.0`, validated from zero to
one) and `bar.decoration.blur` (default `false`). Decoration can cover the whole
bar or just widget islands. See [bar decoration](Bar.md#decoration).
Launcher and finder retain their independent `background_opacity` and `blur`
settings: opacity is clamped to zero through one and defaults to `0.8` with blur,
otherwise `1.0`. Blur requires compositor support.

Implementation: [schema and
defaults](../../../crates/ghost-shell-core/ghost-shell-config/src/config.rs) and
[startup
fallback](../../../crates/ghost-shell-applications/ghost-shell-bar/src/app.rs).

### Icon theme

Set `theme.icons` to an installed icon theme's exact, case-sensitive directory
name. It controls launcher application icons and bar tray icon lookup; it does
not change the Base16 palette or bundled audio/battery icons.

```toml
[theme]
icons = "Papirus"
```

This is a free-form string, not a fixed enum. Examples include `Papirus`,
`Papirus-Dark`, `Papirus-Light`, `Adwaita`, and `hicolor`, when installed.
Use the directory name under `share/icons`, not the package name or the display
name inside `index.theme`. For Home Manager:

```nix
home.packages = [ pkgs.papirus-icon-theme ];
programs.ghost-shell.settings.theme.icons = "Papirus";
```

The theme must be available through the session's icon search paths (including
`XDG_DATA_DIRS`). Lookup uses the selected theme, its inherited themes, and the
library's hicolor/file fallbacks. Missing icons retain the existing application
fallback behavior. Setting a name does not install a theme or manufacture
symbolic icons for applications that do not have them.

When omitted, the launcher uses GTK icon-theme detection (otherwise hicolor).
The tray retains its automatic symbolic search through GTK/hicolor,
Papirus-Dark, Papirus, and Adwaita. An explicit setting replaces that automatic
list. The bar refreshes existing tray icons after changes; reopen the launcher
to use the updated icon theme. Installing a new theme during a running session
may still require restarting the bar to rebuild the icon library's theme index.

## Theming

Each desktop application builds its theme from the shared `config.toml`.
The bar follows file changes while running; other applications load at startup. Define `[theme]` and `[general]` once for a consistent appearance.
Fonts come from `[general]`; background blur and opacity are configured
separately for each surface.

### Appearance mode

```toml
[theme]
mode = "dark"
```

`mode` accepts `"dark"` (the default), `"light"`, and `"system"`. System mode
uses iced's reported system theme and follows appearance changes while running.
The bar applies mode and palette edits live; reopen other applications after
editing their configuration.

### Base16 palettes

Built-in dark and light palettes are used when their tables are omitted. To
replace either palette, supply all sixteen fields in `[theme.dark]` or
`[theme.light]`; individual palette fields do not have deserialization defaults.
Use quoted `"#RRGGBB"` strings with exactly six hexadecimal digits for editor
color swatches. Uppercase and lowercase digits are accepted. Decimal integers
and TOML `0xRRGGBB` integers remain supported for existing configurations. The
capitalization of field names `base0A` through `base0F` matters.

This example reproduces the built-in dark palette:

```toml
[theme.dark]
base00 = "#0a0c10"
base01 = "#272b33"
base02 = "#7a828e"
base03 = "#9ea7b3"
base04 = "#bdc4cc"
base05 = "#f0f3f6"
base06 = "#ffffff"
base07 = "#ffffff"
base08 = "#ffb757"
base09 = "#91cbff"
base0A = "#e09b13"
base0B = "#addcff"
base0C = "#72f088"
base0D = "#dbb7ff"
base0E = "#ff9492"
base0F = "#ffb1af"
```

For Home Manager, use the same strings in `programs.ghost-shell.settings`, for
example `theme.dark.base00 = "#0a0c10";`, alongside the other fifteen colors.
Home Manager preserves strings in generated TOML; numeric values may be emitted
as decimal integers. This color syntax also applies to the greeter's theme.
Wallpaper `bg` remains a numeric `0xRRGGBBAA` value.

For the required UI roles, see the [Base16 color
contract](../development/UI-UX.md#base16-color-contract). For the implementation
of palette mapping, see [theme
mapping](../../../crates/ghost-shell-core/ghost-shell-theme/src/theme.rs). Mode
selection is implemented in the
[application](../../../crates/ghost-shell-applications/ghost-shell-launcher/src/app.rs);
defaults live in the [configuration
schema](../../../crates/ghost-shell-core/ghost-shell-config/src/config.rs).

## Wallpapers

The bar application renders desktop backgrounds on connected displays. The
lockscreen renders its own surfaces using the same wallpaper configuration. PNG,
JPEG and animated GIFs are supported.

### Configuration

Use `[wallpaper]` in `~/.config/ghost-shell/config.toml` for both the desktop
and lock background. Each application loads and renders it independently.

To use a solid background:

```toml
[wallpaper]
bg = 0x101820ff
```

To use an image or animation, replace this example path with a readable file:

```toml
[wallpaper]
path = "/home/alice/Pictures/wallpaper.gif"
```

  | Option | Default      | Behavior                                                       |
  | ------ | ------------ | -------------------------------------------------------------- |
  | `path` | Unset        | Image or GIF path; when present, takes precedence over `bg`    |
  | `bg`   | `0x00000000` | Solid color in numeric `0xRRGGBBAA` form when `path` is absent |

Use an absolute path. The implementation passes the string directly to the
filesystem; shell variables and `~` are not expanded. An unreadable or
unsupported file logs an error and leaves the configured `bg` fallback visible.
Omit `path` to use a solid color. There are no per-output wallpaper settings
yet.

Implementation: [wallpaper
manager](../../../crates/ghost-shell-components/ghost-shell-wallpaper/src/wallpaper.rs).

## Editing Home Manager configuration

When Home Manager owns `config.toml`, edit `programs.ghost-shell.settings` in
your Nix configuration and rebuild; do not edit the generated file. TOML tables
map to Nix attribute sets. For example, `[clock] format = "%H:%M"` is expressed
as `programs.ghost-shell.settings.clock.format = "%H:%M";`. Unknown options are
not validated by the generic TOML generator; check the supported fields here.
