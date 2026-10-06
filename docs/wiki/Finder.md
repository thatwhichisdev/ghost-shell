# Finder

[Wiki](README.md) · [Commands](Commands.md)

The finder uses `fff` to index and search files and directories. The current
search root is `/` and is not configurable through TOML.

```sh
ghost-shell msg finder toggle
```

Type to filter indexed entries; Escape closes the finder. The window prefers
the focused output and falls back to the primary output.

## Configuration

```toml
[finder]
blur = false
background_opacity = 1.0
```

| Option | Default | Behavior |
| --- | --- | --- |
| `blur` | `false` | Requests compositor blur behind the finder |
| `background_opacity` | Automatic | `0.8` with blur, otherwise `1.0`; clamped to `0.0`–`1.0` |

## Current limitations

Opening a selected result is unfinished: the current action logs the selected
entry rather than opening the file or its containing directory. `Ctrl+O` and
`Ctrl+P` are registered for open and preview actions, but should not be treated
as completed file-opening or preview features. Search roots and indexing options
are not exposed in the user configuration.

Implementation: [finder](../../crates/ghost-shell-finder/src/finder.rs),
[search defaults](../../crates/ghost-shell-finder/src/search.rs), and
[result handling](../../crates/ghost-shell-finder/src/view.rs).
