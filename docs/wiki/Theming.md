# Theming

[Wiki](README.md) · [Configuration](Configuration.md)

Ghost Shell builds a shared theme at startup. Fonts come from `[general]`;
background blur and opacity are configured separately for each surface.

## Appearance mode

```toml
[theme]
mode = "dark"
```

`mode` accepts `"dark"` (the default), `"light"`, and `"system"`. At startup,
`"system"` uses GPUI's reported window appearance to choose a palette. The daemon
does not currently register an appearance-change listener to switch themes
while running; restart it to apply a new mode.

## Base16 palettes

Built-in dark and light palettes are used when their tables are omitted. To
replace either palette, supply all sixteen fields in `[theme.dark]` or
`[theme.light]`; individual palette fields do not have deserialization defaults.
Colors are numeric `0xRRGGBB` values, not quoted strings. The capitalization of
`base0A` through `base0F` matters.

This example reproduces the built-in dark palette:

```toml
[theme.dark]
base00 = 0x0a0c10
base01 = 0x272b33
base02 = 0x7a828e
base03 = 0x9ea7b3
base04 = 0xbdc4cc
base05 = 0xf0f3f6
base06 = 0xffffff
base07 = 0xffffff
base08 = 0xffb757
base09 = 0x91cbff
base0A = 0xe09b13
base0B = 0xaddcff
base0C = 0x72f088
base0D = 0xdbb7ff
base0E = 0xff9492
base0F = 0xffb1af
```

For the exact mapping from palette entries to UI roles, see
[`Theme::apply_base16`](../../crates/ghost-shell-theme/src/theme.rs).
Mode selection is implemented in the
[daemon entry point](../../crates/ghost-shell-daemon/src/main.rs); defaults live
in the [configuration schema](../../crates/ghost-shell-config/src/config.rs).
