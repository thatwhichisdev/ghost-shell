# Wallpapers

[Wiki](README.md) · [Lockscreen](Lockscreen.md)

The wallpaper manager renders a shared background on connected displays. The
lockscreen uses the same configured source. PNG and JPEG images and animated
GIFs are supported.

## Configuration

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
file currently causes startup to fail instead of falling back to `bg`. Omit
`path` to use a solid color. There are no per-output wallpaper settings yet.

Implementation: [wallpaper manager](../../crates/ghost-shell-wallpaper/src/wallpaper.rs).
