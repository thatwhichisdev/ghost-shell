# Configuration

[Wiki](README.md) · [First session](Getting-Started.md)

## Location and loading

Desktop applications independently read `$XDG_CONFIG_HOME/ghost-shell/config.toml`,
normally `~/.config/ghost-shell/config.toml`. Each deserializes only its relevant
sections; there is no shared runtime or dependency on another application.
Restart affected applications after edits. Configuration is not live-reloaded.

| Application | Sections read |
| --- | --- |
| Bar | `[general]`, `[theme]`, `[bar."OUTPUT"]`, `[clock]`, `[wallpaper]` |
| Launcher | `[general]`, `[theme]`, `[launcher]` |
| Finder | `[general]`, `[theme]`, `[finder]` |
| Lockscreen | `[general]`, `[theme]`, `[wallpaper]` |

Define `[general]`, `[theme.dark]` and `[theme.light]` once for all desktop
applications. `[wallpaper]` supplies the desktop and lock background. There are
no application-specific theme overrides. The boot [greeter](Greeter.md) retains
its own system-owned `/etc/ghost-shell/greeter.toml`, accessible before login.

Omitted sections and fields use defaults, except that an explicitly supplied
Base16 palette requires all sixteen colors. Invalid configuration logs
`Configuration:` and falls back to that application's defaults. With no `[bar.*]`
entries, the bar appears on every connected output. Unknown sections and fields
are ignored, so a type error in an unrelated application's section does not
prevent another application from reading its own settings. TOML syntax errors
still affect the whole file.

## Migrating to config.toml

Existing combined configurations using `[bar.*]`, `[clock]`, `[finder]`,
`[general]`, `[launcher]`, `[theme]` and `[wallpaper]` work without changes.
If you split your configuration, combine those sections into `config.toml`,
choosing one shared theme, general configuration and wallpaper.

For compatibility, when `config.toml` is absent, each application falls back to
its previous `bar.toml`, `launcher.toml`, `finder.toml` or `lockscreen.toml`.
Files are never merged: an existing `config.toml`, even an empty or invalid one,
takes precedence. If neither file exists, defaults apply.

Home Manager generates the shared file through `programs.ghost-shell.settings`.
Move sections out of `applications.<name>.settings` into that option; the removed
options report a migration error. `applications.<name>.enable` still controls
the bar and lockscreen services. A shared configuration change triggers all enabled services
to restart; lockscreen restart still waits for authentication while locked.
See [installation](Installation.md#home-manager).

Launcher and finder let Niri select the active output. They do not read the bar's primary-output setting.
There are no dedicated sections for other widgets. See [widgets](Widgets.md).

## General settings

```toml
[general]
font_family = "monospace"
font_size = 13.0
```

| Option | Type | Default | Behavior |
| --- | --- | --- | --- |
| `font_family` | String | `"monospace"` | Sets the default UI font family; install your chosen font separately |
| `font_size` | Number | `13.0` | Sets the default UI text size |

## Surface opacity

Bars, the launcher, and the finder each have independent `background_opacity`
settings. Values are clamped to `0.0`–`1.0`. When omitted, opacity is `0.8` for
blurred surfaces and `1.0` otherwise. Transparent bars always use `0.0`.
Blur also requires compositor support.

Bars use `appearance = "blur"`, `"themed"`, or `"transparent"`. If migrating an
older configuration, replace bar `blur = true` with `appearance = "blur"`, and
bar `blur = false` with `appearance = "themed"`. Launcher and finder still use
`blur = true` or `false`.

Implementation: [schema and defaults](../../crates/ghost-shell-core/ghost-shell-config/src/config.rs)
and [startup fallback](../../crates/ghost-shell-applications/ghost-shell-bar/src/app.rs).
