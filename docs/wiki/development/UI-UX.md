# UI/UX

[Wiki](../README.md#development) · [Design Principles and
Architecture](Design-Principles-and-Architecture.md)

These rules guide new interfaces and changes to existing ones. They describe
design expectations; individual configuration pages document current behavior
and limitations, including gaps in keyboard support.

## Visual identity

Keep Ghost Shell restrained, legible, and consistent across applications. Use
clear hierarchy, aligned content, and spacing to separate controls. Avoid visual
noise and decoration that competes with the user's work. Take inspiration from
desktop interfaces without copying assets, branding, or distinctive elements
from other products, including Ghost in the Shell.

Use the shared theme and font settings instead of introducing application-local
palettes or arbitrary font choices. Choose semantic colors through the [theme
implementation](../../../crates/ghost-shell-core/ghost-shell-theme/src/theme.rs).
Check dark and light modes, and retain readable foregrounds on themed, blurred,
and transparent surfaces. Blur is optional compositor behavior; content must
remain understandable when it is unavailable.

The current bar uses a 27 logical-pixel default height, 9-pixel horizontal
insets, and 8-pixel spacing in the start and end groups. Most icons fit an
18-pixel square; the battery uses a 24-pixel icon. Treat these as the existing
visual baseline, not new configuration options. Center icons vertically,
preserve aspect ratios, and truncate long titles without displacing adjacent
controls. See [bar
layout](../../../crates/ghost-shell-applications/ghost-shell-bar/src/lib.rs).

## Interaction and focus

Make the main action clear and keep controls consistent. Use explicit labels for
actions, distinguish selected and disabled states, and avoid relying only on
color to communicate status. Present errors near the affected interaction with
enough information to understand what failed and what can be retried.

Launcher and finder should be ready for typing as soon as they open. Keep input
responsive while discovery or indexing runs. Preserve the current query as
results arrive and keep selection predictable. Do not show working-looking
controls for unimplemented actions; Finder currently cannot open or preview
results.

Use Enter for a supported primary action and Escape to dismiss transient
surfaces. Provide keyboard access to new controls and visible focus. Verify
pointer and keyboard behavior in Niri. The current tray menu dependency lacks
arrow-key navigation; preserve that documented limitation until it is actually
implemented. See [Tray](../configuration/Tray.md).

Launcher and finder exit on dismissal. Repeated invocation currently does not
toggle an existing window. Keep this lifecycle visible in user documentation and
avoid adding background processes to support a transient interface.

## Popups and outputs

Applications own popup surfaces and their lifecycle. Anchor a popup to its
trigger, keep it within the usable output, and make Escape and outside clicks
close ordinary transient menus. Audio adjustments should keep the panel open so
users can make several changes. Closing a bar must also close its popups.

Respect Wayland popup-grab ordering when replacing menus. Measure content before
showing it where possible to avoid a temporary oversized surface. Keep submenu
placement and the native surface size consistent. Existing tray behavior and its
limitations are described in [Tray](../configuration/Tray.md).

Check multiple outputs, hotplug, scaling, long labels, empty results, and large
menus. Do not assume every surface belongs to the bar's primary output. Launcher
and finder let Niri select the active output; bars belong to configured outputs.

## Authentication interfaces

Keep login and unlocking distinct. Lockscreen authenticates the existing
session; greeter starts a new session through greetd. Mask secret input and
never log passwords or secret responses. Present failed authentication clearly
without losing the ability to retry.

Do not apply ordinary Escape-to-dismiss behavior to a session lock. Unlock must
complete authentication and the protocol lifecycle. Greeter prompts follow the
serialized greetd conversation; prevent conflicting actions while a response is
pending. Preserve explicit change-user and recovery behavior.

## Feedback and performance

Show loading, empty, unavailable, and failed states deliberately. Background
work must not block input or leave stale controls suggesting that an unavailable
backend is working. Propagate async operation failures into UI messages instead
of relying solely on logs.

Request redraws when state or animation frames change. Static content should not
run an animation timer. Avoid decorative animation that increases idle work or
makes interaction less immediate. Keep expensive image work off the UI thread
and reuse renderer resources as described in
[architecture](Design-Principles-and-Architecture.md#wallpaper-rendering).

## Reviewing a UI change

Validate the affected flow in Niri, including keyboard and pointer input,
dismissal, focus restoration, output changes, and relevant service failures.
Check light and dark themes, scaling, long content, and loading/empty states.
For authentication changes, also test failed authentication and the appropriate
unlock or session-start handoff. Update the configuration guide with any new
behavior or remaining limitations; do not describe intended behavior as shipped.
