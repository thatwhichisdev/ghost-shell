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

## Transitional application support

The daemon still uses upstream GPUI and toolkit components. Only it enables
`ghost-shell-theme/legacy`, which preserves the previous theme initializer
in `legacy.rs`. This adapter is existing Ghost code, not the new extraction.
It is not a bridge between GPUI entity types and must be removed when the
daemon and remaining widgets migrate. New components must not enable it.

The unused old `src/lib.rs` theme entry point was replaced by `src/theme.rs`.
The public local GPUI manifest was also repaired to use the existing
Wayland facade instead of the stale original upstream core manifest.

## Verification

Run these inside Ghost's development environment:

```sh
cargo test -p ghost-shell-theme -p ghost-shell-component-root
cargo check -p ghost-shell-component-root --example hello_components
cargo check -p ghost-shell-daemon
cargo run -p ghost-shell-component-root --example hello_components
```

The example opens a normal Wayland window; it does not start or replace
the running shell.
