# Wallpapers

[Wiki](README.md) · [Lockscreen](Lockscreen.md)

The bar application renders desktop backgrounds on connected displays. The
lockscreen renders its own surfaces using the same wallpaper configuration. PNG, JPEG and animated GIFs
are supported.

## Configuration

Use `[wallpaper]` in `~/.config/ghost-shell/config.toml` for both the desktop and
lock background. Each application loads and renders it independently.

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

| Option | Default | Behavior |
| --- | --- | --- |
| `path` | Unset | Image or GIF path; when present, takes precedence over `bg` |
| `bg` | `0x00000000` | Solid color in numeric `0xRRGGBBAA` form when `path` is absent |

Use an absolute path. The implementation passes the string directly to the
filesystem; shell variables and `~` are not expanded. An unreadable or unsupported
file logs an error and leaves the configured `bg` fallback visible. Omit
`path` to use a solid color. There are no per-output wallpaper settings yet.

Implementation: [wallpaper manager](../../crates/ghost-shell-components/ghost-shell-wallpaper/src/wallpaper.rs).

## Animation rendering

GIFs are decoded on a worker and stored as an initial RGBA frame plus compressed
sparse changes, including the transition back to the first frame. A versioned
cache is checked against the source path, size, and modification time. Each surface has reusable playback buffers and a persistent
GPU texture rendered through an iced shader with cover scaling. Frame deadlines
request redraws locally; static images do not run an animation timer.

Very short GIF delays are clamped to 10 ms. After a long pause, playback resumes
with bounded catch-up while preserving the delta sequence. Images and animations
have memory and dimension limits; unsupported sources leave the fallback visible.
