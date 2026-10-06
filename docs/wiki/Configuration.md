# Configuration

[Wiki](README.md) · [First session](Getting-Started.md)

## Location and loading

Ghost Shell reads TOML from `$XDG_CONFIG_HOME/ghost-shell/config.toml`, normally
`~/.config/ghost-shell/config.toml`. The daemon loads it at startup; restart it
after changes. There is no config-path CLI option or live reload.

Omitted sections and fields use defaults. If the file is missing or cannot be
loaded or deserialized, the daemon prints `Failed to load config` and uses the
entire default configuration. Defaults contain no bars. Unknown fields are not
rejected by the current schema, so a misspelled option may silently do nothing.

## Sections

| Section | Purpose | Reference |
| --- | --- | --- |
| `[general]` | Font family and size | Below |
| `[bar."OUTPUT"]` | One bar per configured output | [Bar](Bar.md) |
| `[launcher]` | Launcher blur and opacity | [Launcher](Launcher.md) |
| `[finder]` | Finder blur and opacity | [Finder](Finder.md) |
| `[clock]` | Shared bar clock format | [Clock](widgets/Clock.md) |
| `[wallpaper]` | Shared desktop and lockscreen background | [Wallpapers](Wallpapers.md) |
| `[theme]`, `[theme.dark]`, `[theme.light]` | Mode and palettes | [Theming](Theming.md) |

There are no dedicated TOML sections for the other widgets or the lockscreen.
See the [widget reference](Widgets.md) before adding settings.

## General settings

```toml
[general]
font_family = "monospace"
font_size = 13.0
```

| Option | Type | Default | Behavior |
| --- | --- | --- | --- |
| `font_family` | String | `"monospace"` | Sets the theme's sans font family; install your chosen font separately |
| `font_size` | Number | `13.0` | Sets the theme's medium typography size |

## Surface opacity

Bars, the launcher, and the finder each have independent `background_opacity`
settings. Values are clamped to `0.0`–`1.0`. When omitted, opacity is `0.8` for
blurred surfaces and `1.0` otherwise. Transparent bars always use `0.0`.
Blur also requires compositor support.

Bars use `appearance = "blur"`, `"themed"`, or `"transparent"`. If migrating an
older configuration, replace bar `blur = true` with `appearance = "blur"`, and
bar `blur = false` with `appearance = "themed"`. Launcher and finder still use
`blur = true` or `false`.

Implementation: [schema and defaults](../../crates/ghost-shell-config/src/config.rs)
and [startup fallback](../../crates/ghost-shell-config/src/lib.rs).
