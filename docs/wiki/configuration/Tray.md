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
with submenus expanding leftward. Existing menus stay aligned to the current
surface's right edge while Wayland applies a size change, avoiding a transient
jump outside the popup when submenus open or close. Escape or clicking outside
closes the popup.

The current iced_aw menu release does not provide arrow-key navigation. Use the
pointer to select actions and open submenus.

Ghost first tries a symbolic SVG from the installed icon themes, using the
reported icon name (or its filename for absolute paths), then the tray item's
identifier. It keeps an existing `-symbolic` suffix or appends one. Lookup uses [general.theme](Introduction.md#icon-theme) when set. Otherwise it
tries the GTK icon theme, Papirus-Dark, Papirus, and Adwaita, including the
lookup library's inherited-theme fallbacks. Successful replacements use the
bar's `base07` foreground. This is a best-effort visual experiment: applications
without a matching symbolic SVG retain their original icons, and a generic
application replacement may omit badges or other image-encoded status. Papirus
panel icons without symbolic names are not automatically matched. Themes must
be installed locally; restart the bar after installing a theme. Debug logging
reports successful replacement paths.

Icons can come from application pixmaps, absolute file paths, or named icons in
the desktop icon theme (with hicolor fallback). The application’s icon directory
is checked first when supplied. Both raster images and SVG icons are supported,
including GlobalProtect’s symbolic status icons. Icons fit within a 16px
square and preserve their aspect ratio. They are
centered vertically in the bar. Hovering a tray icon highlights its inner container while preserving the
original fallback colors; symbolic replacements use `base07`. No tooltip is shown. Menus use a `base00` background and a 1px `base03` border with 6px
rounded corners. Highlighted items use `base07` for both fill and outline, with
`base00` text.

Each option has a minimum 30px row height and grows to fit wrapped labels. Its
selection fill and border are inset by 4px on every side and use a 4px corner
radius, leaving a small gap between adjacent highlights. Labels are vertically
centered inside the highlight, with no additional vertical button padding. This
styling also applies to submenu options. Separators occupy 15px rows, with
the thin divider centered vertically and inset horizontally.

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
