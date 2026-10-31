# Live styling

[Wiki](../README.md#configuration) · [Bar](Bar.md) · [Theme](Introduction.md#theming)

Click the system icon in the bar's start section and choose **Appearance**. The
panel previews changes on every connected bar immediately. Close it with
**Close**, Escape, or an outside click; closing keeps the current preview.
System, audio, tray menus, and styling share one popup slot.

Controls cover:

- Bar height and reserved space.
- Continuous/islands layout, background opacity, compositor blur, and radius.
- Widget height, icon size, horizontal padding, and horizontal/vertical margins.
- Font family and size for bar content and audio/tray menus.
- Dark/light/system theme mode and icon theme.
- All sixteen Base16 colors in either palette, with swatches.

Sliders preview continuously. Enter a font family or installed icon theme and
press its **Apply** button or Enter. An empty icon theme restores automatic
lookup. Font and theme installation remain separate system configuration.
The panel uses `base08` for control accents, including buttons and sliders.
Palette swatches still show their actual configured colors.
The editor keeps startup typography so very large bar-font previews do not
make the recovery controls unusable.

Choose **Edit dark palette** or **Edit light palette** to select which palette
to change. To see that palette immediately, select the corresponding appearance
mode. Color entries accept `#RRGGBB`; incomplete or invalid input displays an
error and preserves the last applied color. Background/foreground roles remain
the fixed theme mapping; the panel edits the palette itself.

**Reset from file** restores the most recent successfully loaded configuration.
**Copy Nix** copies a `programs.ghost-shell.settings = { ... };` snippet containing
the applied general, bar, and theme settings, including both complete palettes
and the configured outputs. Merge those sections into your existing Nix source;
retain your launcher, finder, clock, wallpaper, and other settings. Unsubmitted
font/icon text and incomplete color edits are not exported.

Preview changes stay in the bar process. They do not write `config.toml`, change
other applications, or survive a bar restart. Persist them in your Nix config
and rebuild. Home Manager's new module lets the bar reload the generated file
without restarting the service; updating the executable itself can still
restart it. Launcher and finder use the persisted theme on their next launch.

The bar also reloads direct file edits automatically. It watches directories so
atomic saves and Nix symlink replacements are detected, debouncing event bursts
for 150ms. Relevant valid changes replace the current preview and reset the
panel fields. Invalid or removed files retain working settings: **Appearance!** in the system menu marks
the error, and the panel explains it. A failed wallpaper load keeps the current
wallpaper and reports an error. Installing an entirely new icon theme may
require restarting the bar to refresh the icon library's installed-theme index.

Compositor blur depends on protocol support. The panel requests blur for the
configured decoration regions, including islands; it cannot set blur strength.
The popup fits the output's reported logical size and scrolls when its controls
do not fit vertically. Live interaction and compositor behavior still need
validation in Niri after changes.

Implementation: [styling widget](../../../crates/ghost-shell-widgets/ghost-shell-widget-styling/src/styling.rs),
[config watcher](../../../crates/ghost-shell-core/ghost-shell-config/src/watch.rs),
[bar runtime](../../../crates/ghost-shell-applications/ghost-shell-bar/src/app.rs).
