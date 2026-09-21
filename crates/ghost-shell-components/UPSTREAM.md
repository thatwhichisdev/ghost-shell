# Component extraction

Source: https://github.com/longbridge/gpui-kit

Revision: `0e63ea799766c486022a0cecfda6e48c5183a2d7` — the tip of
`main` retrieved on 2026-09-20. Future extractions should record their
source revision rather than silently following a moving branch.

Copyright 2024 - 2026 Longbridge <https://longbridge.com>.
The extracted portions remain Apache-2.0 licensed; see the repository
`LICENSE.md` and `NOTICE`.

## Sources and changes

| Local source | Upstream source | Adaptation |
| --- | --- | --- |
| `../ghost-shell-theme/src/theme.rs` | `crates/base/src/theme.rs`, `crates/component/src/theme/mod.rs` | One global theme and one semantic token store; no registry, editor settings, or component-specific overlays. |
| `../ghost-shell-theme/src/tokens.rs` | `crates/base/src/theme_tokens.rs` | Semantic colors, typography, spacing, radii, shadows; Linux monospace default only. |
| `../ghost-shell-theme/src/sizing.rs` | `crates/component/src/sizing.rs` | Size, Sizable, input padding and row height only. |
| `../ghost-shell-theme/src/focus.rs` | `crates/base/src/focus_trap.rs` | Idempotent initialization, checked layout ID, innermost trap selection. |
| `ghost-shell-component-root/src/root.rs` | `crates/component/src/root.rs` | Theme styling and tab traversal; restore focus if bounded traversal cannot stay in a trap. |
| `ghost-shell-component-icon` | `crates/component/src/icon.rs`, `crates/assets` | SVG paths/data and a small embedded icon set; no asset-loader dependency. |
| `ghost-shell-component-spinner` | `crates/component/src/spinner.rs` | Local icons and GPUI animation, including reduced-motion behavior. |
| `ghost-shell-component-menu` | `crates/component/src/menu/popup_menu.rs`, `menu_item.rs` | Local theme and icons, GPUI layout tracking, keyboard/submenu dismissal; no native menus, shortcut badges, or external scrollbar component. |
| `ghost-shell-component-virtual-list` | `crates/base/src/virtual_list.rs` | Variable-size virtualization and deferred scrolling; no toolkit scrollbar trait. |
| `ghost-shell-component-input` | `crates/component/src/input/input.rs`, `crates/base/src/input/base/{state,selection,blink_cursor,element}.rs` | Single-line editing, password protection, UTF-16 IME bridge and bounded undo history; no code editor, LSP, textarea, native menu or touch-input dependencies. |

Input's low-level element implementation also adapts the GPUI input example
from Zed revision `916fc2b8cb3a815cbef4a3b40e13081be72036b6`
(`crates/gpui/examples/input.rs`, Apache-2.0). It uses the local framework's
text shaping and input-handler APIs. This is a narrowed adaptation, not the
complete GPUI Kit input API. Password values are excluded from clipboard,
accessibility values, and undo history.

Embedded SVGs retain the upstream Lucide/Feather notices in
`ghost-shell-component-icon/LICENSE-LUCIDE`.

These crates depend on `ghost-shell-gpui` and import it as
`ghost_shell_gpui`. Its public facade re-exports the local core types.
They do not depend on GPUI Kit or its base crate.

Root deliberately has no client-side decoration wrapper: Ghost currently
disables this wrapper on all toolkit roots. It also has no input entity,
dialog/sheet/toast state, rich-text selection, tooltip/native-menu overlay,
or mobile touch-selection layer. Later components should own their state
without introducing a Root–Input dependency cycle.

Theme exposes upstream's semantic tokens rather than copying its large
legacy component-specific color catalogue. `Theme::set` refreshes windows;
Root also observes global theme changes. Base16 palette values can be
applied without depending on the application's configuration crate.

## Application integration

The daemon and widgets now use the local framework and component crates.
The temporary `ghost-shell-theme/legacy` adapter has been removed.
The daemon applies its configured font and Base16 palette to the local Theme;
components remain independent of application configuration.

The unused old `src/lib.rs` theme entry point was replaced by `src/theme.rs`.
The public local GPUI manifest was also repaired to use the existing
Wayland facade instead of the stale original upstream core manifest.

## Verification

Run these inside Ghost's development environment:

```sh
cargo test -p ghost-shell-theme -p ghost-shell-component-root
cargo test -p ghost-shell-component-input -p ghost-shell-component-menu -p ghost-shell-component-icon -p ghost-shell-component-spinner -p ghost-shell-component-virtual-list
cargo check -p ghost-shell-component-root --example hello_components
cargo check -p ghost-shell-daemon
cargo run -p ghost-shell-component-root --example hello_components
```

The example shows inputs, password masking, a spinner, icons, a nested menu,
and a variable-height list in a normal Wayland window; it does not start or
replace the running shell.
