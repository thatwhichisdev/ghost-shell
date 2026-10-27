# Tray

[Wiki](../README.md#configuration) · [Introduction](Introduction.md) ·
[Bar](Bar.md)

Status: **Implemented with incomplete interactions.**

Displays applications registered through the session's StatusNotifier D-Bus
integration. Left-click an item to open its exported menu when it provides one.
Tray and audio popups share one active popup slot across outputs. Opening
another waits for the previous popup's close notification before creating its
replacement, so overlapping requests do not violate Wayland's popup grab
ordering. The menu layout loads before its popup is created, so the first frame
uses the measured menu size without a temporary oversized window. The popup
content uses iced_aw's `MenuBar`, `Menu`, and `Item` widgets. Ghost maps the
exported D-Bus layout to a menu tree; iced_aw handles submenu hover behavior,
scrolling, and rendering. Only open menus contribute to the native popup size,
with submenus expanding leftward. Escape or clicking outside closes the popup.

The current iced_aw menu release does not provide arrow-key navigation. Use the
pointer to select actions and open submenus.

Icons can come from application pixmaps, absolute file paths, or named icons in
the desktop icon theme (with hicolor fallback). The application’s icon directory
is checked first when supplied. Both raster images and SVG icons are supported,
including GlobalProtect’s symbolic status icons. Icons fit within an 18px
square, matching the menu icon, and preserve their aspect ratio. They are
centered vertically in the bar. Hovering a tray icon shows no tooltip or
highlight. Menus have a 1px white border with 6px rounded corners; selected
items use the theme's primary color with black text.

## Setup and configuration

Run tray applications in the same session D-Bus environment as Ghost Shell. The
tray is included automatically in the end section of each bar; there are no
`[tray]` options for ordering, filtering, or sizing.

Ghost hosts the StatusNotifier watcher when its D-Bus name is available. If
another bar already owns the watcher, Ghost registers as a host, loads the
existing tray items and follows registration changes. This allows multiple bars
to display the same tray applications side by side. When the external watcher
exits, Ghost clears its old registrations and attempts to host the watcher
itself.

## Current limitations

The current widget wires left-click to a D-Bus menu when present. It does not
provide general activation, secondary-click, or scroll actions for items without
that menu. Application support therefore depends on the menu the item exports.

Implementation: [widget
crate](../../../crates/ghost-shell-widgets/ghost-shell-widget-tray/src/tray.rs).
