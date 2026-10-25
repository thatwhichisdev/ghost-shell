# Bar and outputs

[Wiki](README.md) · [Configuration](Configuration.md) · [Widgets](Widgets.md)

The bar is a top-anchored Wayland layer-shell surface. Without `[bar.*]` entries,
a default bar opens on every connected output, including newly connected displays.
Explicit entries restrict bars to the named outputs. Configure a separate bar
for each output on which it should appear in `~/.config/ghost-shell/config.toml`:

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
| `primary` | `false` | Preferred fallback output within the bar application |
| `appearance` | `"themed"` | `"themed"` (alias `"default"`), `"blur"`, or `"transparent"` |
| `background_opacity` | Automatic | `1.0` for themed, `0.8` for blur, always `0.0` for transparent |

Themed bars use the active palette's `base00` background without blur. Blur uses
the theme background and asks the compositor to blur behind it. Transparent
bars retain visible widgets without a background or blur.

The primary-output setting is local to the bar application. Launcher and finder
use their own focused-output selection. Bars reconcile when displays change;
a disconnected output's bar is removed and can return on reconnection.

## Widget layout

| Section | Widgets, in order |
| --- | --- |
| Start | System menu, workspaces |
| Center | Focused window |
| End | Tray, audio, power, clock |

Bar content has a 9px inset on both the left and right edges.
The menu, workspace dots and speaker use 18px icons; the battery uses a 24px
state icon. Icons are vertically centered, with filled and outlined dots marking
active and inactive workspaces respectively.
This layout is currently fixed in code. TOML cannot add, remove, reorder, or
independently configure widget instances per output. Network and notifications
are not rendered. See [widgets](Widgets.md) for current behavior and limits.

Implementation: [bar manager](../../crates/ghost-shell-applications/ghost-shell-bar/src/lib.rs),
[layout](../../crates/ghost-shell-applications/ghost-shell-bar/src/lib.rs), and
[output selection](../../crates/ghost-shell-applications/ghost-shell-bar/src/app.rs).
