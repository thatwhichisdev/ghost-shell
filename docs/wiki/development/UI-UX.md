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

Bar styling is global across outputs: `[bar]` controls geometry, `[bar.widgets]`
controls widget dimensions, and `[bar.decoration]` controls continuous or island
backgrounds, opacity, blur and rounding. Defaults are 28px bar height, 20px
inner height, 16px icons, 4px horizontal padding, 2px horizontal/4px vertical
outer margins and 4px corner radius. All sections share these dimensions.
Joined tray/audio items round only outside edges and share one painted island
background and blur region; individual items paint only their hover highlight.
Workspaces share one island.

Continuous mode decorates the full bar; islands decorate only widget inner
containers. Resting backgrounds use `base00` at configured opacity, contents
use `base07`, and hover highlights use `base02`. Original tray icons retain
their colors. Compositor blur regions follow decoration bounds, approximating
rounded corners with integer rectangles and preserving clear gaps. No new
background, foreground or hover-color fields are exposed. The icon theme is
`theme.icons`; `[general]` owns fonts only. See [bar configuration](../configuration/Bar.md).

## Base16 color contract

Ghost uses its own UI mapping of the Base16 palette. The slot
names remain compatible with configuration. Menus use inverted selection;
launcher and finder result rows use `base02` selection fill. These rules apply to both dark and light polarity:
select the whole configured palette without reversing or renumbering slots.

### Surfaces and selection

- Application surfaces, bars, audio panels, and tray menus use `base00`.
- Ordinary text and monochrome icons use `base05`. Bar inner
  containers use `base07` foreground, with `base00` island backgrounds or
  transparent resting backgrounds in continuous mode, and `base02` on hover.
- Highlighted tray options use `base07` for
  both background and their 1px border, with `base00` text.
- Keep this selected fill, border, and text in active, hovered, and pressed
  states. Do not substitute a primary-accent border or choose a different text
  color automatically. Disabled selected controls may reduce text opacity.
- Tray option highlights have a 4px inset on all sides and a 4px corner radius
  within rows at least 30px tall. Grow rows to fit wrapped labels. Apply the
  inset to every option, including submenu parents, so highlighting does not
  shift layout. Center labels vertically in the remaining button height and
  avoid additional vertical button padding.
- Tray menu outer borders remain `base03`; this is distinct from an option's
  selection border.

Launcher and finder windows use `ghost_shell_theme::search_window`: a 4px
corner radius and 1px `base07` border over the configured surface background.
Their search inputs use `search_input`, with `base07` borders and 4px rounded corners in every state.
Their result rows use `search_item`: `base02` with 4px rounded corners when
selected (including hover),
`base01` when only hovered or pressed, and transparent otherwise. Text remains
`base05`; selected rows do not have an inverted fill or border.

Use `ghost_shell_theme::surface` for application backgrounds,
`ghost_shell_theme::bar` for bar decorations, `ghost_shell_theme::panel` for
panels, and
`ghost_shell_theme::selected` for inverted menu selections. Both
surface helpers use `base00`; configured opacity and transparency still apply.
Use `ghost_shell_theme::text_input` for text fields.

Tray and audio menu separators use 15px rows with vertically centered thin
lines. Audio device groups have separators between them. Their control rows
place an `M` mute toggle before the slider; muted toggles use the selected
style and follow confirmed backend state. Audio menu widths follow rendered
device names plus padding, bounded to keep controls usable and long names
wrappable.

Audio sliders use `base07` for the filled track and handle in every interaction
state, and `base03` for the unfilled track. Active audio device names use the
selected-row background without an `(active)` prefix.

### Slot roles

Choose slots by meaning, not by their current hue. For example, `base08` remains
an error color even if a custom palette makes it orange, and `base0D` remains a
primary-action color even if it is purple.

  | Slot     | Ghost UI role                                 | Current mapping or use                                                                       |
  | -------- | --------------------------------------------- | -------------------------------------------------------------------------------------------- |
  | `base00` | Surface background; inverted selection text   | iced `Palette.background`; surfaces, panels, menus, selected-row text                        |
  | `base01` | Subtle neutral control background             | Extended `background.weaker`, `background.weak`, and `secondary.weak`; not panel backgrounds |
  | `base02` | Text selection and secondary control emphasis | Extended `secondary.strong`; not row/menu selection                                          |
  | `base03` | Muted hints and structural detail             | Input placeholders, unfocused input borders, tray outer borders                              |
  | `base04` | Secondary foreground and subdued status       | Extended `secondary.base.color`; disabled input values and pick-list placeholders            |
  | `base05` | Default foreground                            | iced `Palette.text`; body text, ordinary labels, monochrome icons                            |
  | `base06` | Rare high-emphasis neutral detail             | Extended `background.strongest.color`                                                        |
  | `base07` | Inverted selection fill and border            | Extended `background.neutral.color`; selected rows and highlighted menu options              |
  | `base08` | Errors and destructive actions                | iced `Palette.danger`; error messages use `text::danger`                                     |
  | `base09` | Constants and literal values                  | Reserved; no iced palette slot                                                               |
  | `base0A` | Warnings and search-match emphasis            | iced `Palette.warning`; search-match emphasis reserved                                       |
  | `base0B` | Success and positive confirmation             | iced `Palette.success`                                                                       |
  | `base0C` | Supporting information                        | Reserved; no iced palette slot                                                               |
  | `base0D` | Primary actions and control focus             | iced `Palette.primary`; not selected-row borders                                             |
  | `base0E` | Keywords and categorical emphasis             | Reserved; no iced palette slot                                                               |
  | `base0F` | Deprecated content                            | Reserved; no iced palette slot                                                               |

Reserved roles have no current UI consumer. Add shared semantic accessors when
introducing them rather than borrowing another iced slot. Application-supplied
icons, wallpaper, and other image content retain their own colors, including
original tray icons in both idle and hovered states. Explicit symbolic tray
replacements use the bar foreground color.

### iced extended palette

The six direct iced fields map to `base00`, `base05`, `base0D`, `base0B`,
`base0A`, and `base08`, respectively. The shared theme supplies these explicit
neutral pairs:

  | Extended field                          | Color    | Foreground                             |
  | --------------------------------------- | -------- | -------------------------------------- |
  | `background.base`, `background.weakest` | `base00` | `base05`                               |
  | `background.weaker`, `background.weak`  | `base01` | `base05`                               |
  | `background.neutral`                    | `base07` | `base00`                               |
  | `background.strong`                     | `base03` | `base05`                               |
  | `background.stronger`                   | `base04` | `base05`                               |
  | `background.strongest`                  | `base06` | `base00`                               |
  | `secondary.base`                        | `base04` | Contrast choice of `base00` / `base07` |
  | `secondary.weak`                        | `base01` | `base05`                               |
  | `secondary.strong`                      | `base02` | Contrast choice of `base00` / `base07` |

These are iced compatibility slots, not interchangeable surface styles. Accent
`base` colors stay exact. iced 0.14 derives each accent's `weak` color by mixing
in 40% background, and its `strong` color with its lightness-deviation algorithm
(amount 0.1). These variants are permitted for standard control hover/drag
states. Accent-pair text uses whichever of `base00` and `base07` has higher
relative contrast. This automatic choice applies to accent controls, not
inverted row/menu selections, whose text is always `base00`.

### Contrast and exceptions

Do not hard-code black or white for ordinary text, borders, or selection. Use
the configured slots so the same rules also work with light themes. Choose
`base00` and `base07` with strong contrast and check custom palettes visually.

iced 0.14 text inputs have a single value color for selected and unselected
text. They therefore retain `base02` for text selection and `base05` for glyphs;
using the inverted row fill without an independent selected-text color would
make text unreadable. Choose those two slots with sufficient contrast. This
exception is limited to text inputs, not inverted menu items.

Transparent Wayland clear colors and image-rendering fallbacks are rendering
concerns, not semantic UI colors. Transparency and wallpaper can affect
perceived contrast and require compositor testing.

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

Launcher and finder hovered result backgrounds use the same 4px rounding as
selected results. Hovered, pressed, and dragged scrollbar handles use `base08`.

## Live styling

The system menu's **Appearance** action opens an ordinary popup shared with system,
audio and tray menus. It closes with Escape or an outside click. Live edits affect all bars;
closing the popup keeps the preview. Keep reset and Nix-copy actions explicit,
leave Nix-managed files untouched, and retain working settings after an invalid
file reload. Incomplete color text must remain editable without applying an
invalid palette. Feedback appears at the top of the panel, and reload errors
also change the system menu entry to **Appearance!**. The styling panel uses
`base08` in place of `base0D` for control accents; palette swatches and exported
values keep their original slots. Its controls use stable startup typography so large font
previews remain recoverable.
