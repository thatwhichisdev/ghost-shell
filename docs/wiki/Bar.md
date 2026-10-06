# Bar and outputs

[Wiki](README.md) · [Configuration](Configuration.md) · [Widgets](Widgets.md)

The bar is a top-anchored Wayland layer-shell surface. Configure a separate bar
for each output on which it should appear:

```toml
[bar."eDP-1"]
output = "eDP-1"
height = 27.0
exclusive_zone = 27.0
appearance = "themed"

[bar."DP-1"]
output = "DP-1"
primary = true
height = 27.0
exclusive_zone = 27.0
appearance = "blur"
background_opacity = 0.8
```

Use `niri msg outputs` to find output names. The **table key**, such as `DP-1`,
is what the current implementation matches to a connected display. The `output`
field exists in the schema but does not select the display; keep it identical
to the table key. Arbitrary table names such as `[bar.main]` will not match `DP-1`.

| Option | Default | Behavior |
| --- | --- | --- |
| `output` | `"<default>"` | Schema field; output selection currently uses the table key |
| `height` | `27.0` | Bar height in logical pixels |
| `exclusive_zone` | `27.0` | Space reserved for the bar in logical pixels |
| `primary` | `false` | Preferred fallback output for shell surfaces |
| `appearance` | `"themed"` | `"themed"` (alias `"default"`), `"blur"`, or `"transparent"` |
| `background_opacity` | Automatic | `1.0` for themed, `0.8` for blur, always `0.0` for transparent |

Themed bars use the active palette's `base00` background without blur. Blur uses
the theme background and asks the compositor to blur behind it. Transparent
bars retain visible widgets without a background or blur.

Set one connected output as primary. If multiple bars set `primary = true`,
the lexicographically first table key is selected. If that display is absent,
the shell falls back to an available display. Launcher and finder prefer the
focused output, then the primary output. Bars reconcile when displays change;
a disconnected output's bar is removed and can return on reconnection.

## Widget layout

| Section | Widgets, in order |
| --- | --- |
| Start | System menu, workspaces |
| Center | Focused window |
| End | Tray, audio, power, clock |

This layout is currently fixed in code. TOML cannot add, remove, reorder, or
independently configure widget instances per output. Network and notifications
are not rendered. See [widgets](Widgets.md) for current behavior and limits,
including the battery requirement that can prevent all bars from appearing.

Implementation: [bar manager](../../crates/ghost-shell-bar/src/lib.rs),
[layout](../../crates/ghost-shell-bar/src/bar.rs), and
[output selection](../../crates/ghost-shell-app/src/lib.rs).
