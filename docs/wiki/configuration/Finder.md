# Finder

[Wiki](../README.md#configuration) · [Introduction](Introduction.md)

The finder uses `fff` to index and search files and directories. The current
search root is the current user's home directory, resolved at runtime, and is
not configurable through TOML. Filesystem-root scanning and following symlinks
are disabled. Each invocation builds its own in-memory index; no persistent
index is implemented yet.

```sh
ghost-shell-finder
```

The window opens while indexing runs in the background. You can type
immediately; pressing Enter during indexing queues a search using the latest
query once the scan finishes. Escape or surface closure terminates the process
and releases the index. Niri chooses the active output for the centered overlay,
which takes exclusive keyboard focus. A process lock prevents duplicate windows.
There is no finder service or IPC server.

Windows have 4px rounded corners and a 1px `base07` border. Search input
borders also use `base07` with 4px rounded corners. Selected results have 4px rounded corners and use `base02`, including when
hovered; other hovered results use `base01`. Result text uses `base05`.

## Configuration

Put these settings in `~/.config/ghost-shell/config.toml`.

```toml
[finder]
blur = false
background_opacity = 1.0
```

  | Option               | Default   | Behavior                                                 |
  | -------------------- | --------- | -------------------------------------------------------- |
  | `blur`               | `false`   | Requests compositor blur behind the finder               |
  | `background_opacity` | Automatic | `0.8` with blur, otherwise `1.0`; clamped to `0.0`–`1.0` |

## Current limitations

Opening and previewing a selected result remain unimplemented. The former
placeholder Ctrl+O/Ctrl+P actions are not registered. Search roots and indexing
options are not exposed in the user configuration.

Implementation:
[finder](../../../crates/ghost-shell-applications/ghost-shell-finder/src/finder.rs),
[search
defaults](../../../crates/ghost-shell-applications/ghost-shell-finder/src/search.rs),
and [result
handling](../../../crates/ghost-shell-applications/ghost-shell-finder/src/finder.rs).

Arrow-key navigation scrolls only when the selected row leaves the visible
list, moving just enough to reveal it.
Paths wrap at the available width. Each line gets 24px of row height, so
two-line results occupy 48px and longer paths grow further without overlapping.
The result list lays out at most the 100 returned search results to measure
these variable row heights.

Hovered result backgrounds also have 4px rounded corners. Hovering, pressing, or
dragging the scrollbar colors its handle `base08`.
