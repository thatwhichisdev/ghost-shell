# Launcher

[Wiki](README.md) · [Commands](Commands.md)

The launcher discovers desktop application entries and filters them as you type.
It launches the selected entry through Niri's spawn command. Desktop entries
marked as terminal applications currently run through `ghostty -e`; the terminal
choice is hard-coded and cannot yet be changed in TOML.

```sh
ghost-shell msg launcher toggle
```

Press Enter to launch the selected application and Escape to close the launcher.
The window opens on the focused output, falling back to the primary output.

## Configuration

```toml
[launcher]
blur = true
background_opacity = 0.8
```

| Option | Default | Behavior |
| --- | --- | --- |
| `blur` | `false` | Requests compositor blur behind the launcher |
| `background_opacity` | Automatic | `0.8` with blur, otherwise `1.0`; clamped to `0.0`–`1.0` |

The [general font and theme](Theming.md) also apply. There are no launcher-specific
terminal, application-directory, or key-binding settings in the TOML schema.

Implementation: [launcher](../../crates/ghost-shell-launcher/src/launcher.rs),
[desktop discovery](../../crates/ghost-shell-launcher/src/entries.rs), and
[launch action](../../crates/ghost-shell-launcher/src/view.rs).
