# GPUI extraction

Imported from Zed commit `916fc2b8cb3a815cbef4a3b40e13081be72036b6`
(local checkout: zed-industries/zed). Original framework and helper code is
Apache-2.0; see LICENSE-APACHE. Font fixtures carry their own licenses under
test_assets. Keep this provenance when making subsequent changes.

## Crate boundaries

```text
ghost-shell-gpui (public entry point and Linux/Wayland backend)
├── ghost-shell-gpui-core (entities, UI, layout, input, platform traits)
│   ├── ghost-shell-gpui-macros (separate proc-macro crate)
│   └── support/* (collections, scheduler, refineable, sum_tree)
└── ghost-shell-wgpu ──> ghost-shell-gpui-core

ghost-shell-tokio ──> ghost-shell-gpui-core
ghost-shell-util (shared helpers)
```

The internal core manifest points at ../src/gpui.rs. This preserves the copied
source layout while avoiding a core → renderer → core dependency cycle.
SharedString is a core module, not another crate. The public entry point
reexports the core API, so its entities and the renderer/Tokio types agree.

## Scope

Kept: Wayland and layer-shell, popups, Vulkan/OpenGL rendering, local/embedded
images, SVG, text shaping/fonts, IME, clipboard/drag-and-drop, accessibility,
desktop portals, system notifications, headless tests and optional inspection
and profiling tools. These support shell interfaces, not just editors.

Removed: X11, Windows/macOS platform branches, browser/Wasm backend code,
screen capture, macOS video surfaces and native window tabs, HTTP clients and
remote image fetching, and Zed's GitHub-account keyring integration.
URI image requests now return UnsupportedUri; use filesystem paths, embedded
assets or in-memory images instead. The local image dependency enables common
UI formats without the default AVIF encoder and specialist image codecs.

No dependency on the Zed monorepo is required by these local crates. External
libraries remain ordinary dependencies, including the pinned zed-font-kit fork
and the proptest fork used for tests. The imported sum_tree uses tracing directly,
without Zed's ztracing/zlog dependency chain.

The shell now uses the local framework, Tokio integration, theme, and component
crates throughout. No upstream GPUI or GPUI Kit dependency remains.

The application migration also ports these Ghost fork patches from
`thatwhichisdev/zed` (previous dependency revision `3c0656db8f7fd19f1d7422db7eeafd3b8d491d52`):

- `aa0c1b6acd`: discover initial Wayland outputs before shell startup.
- `aab23cce3c`, `b127471ed9`: session-lock surfaces and deferred unlock cleanup,
  adapted to the local frame scheduling API.
- `7abb543947`: in-place wallpaper atlas updates, adapted to `AtlasState` and
  strengthened with checked bounds and byte-length validation.

Session locking still requires compositor-level testing; headless tests do not
establish lockscreen security or multi-output behavior.

## Verification

Run inside the project's Nix development environment:

```sh
cargo build -p ghost-shell-gpui -p ghost-shell-wgpu -p ghost-shell-tokio
cargo test -p ghost-shell-gpui-core -p ghost-shell-wgpu -p ghost-shell-gpui --lib
cargo check -p ghost-shell-gpui -p ghost-shell-wgpu -p ghost-shell-tokio --all-targets --all-features
cargo run -p ghost-shell-gpui --example hello_shell
```

The last command needs a running Wayland compositor and a working GPU driver.
The unit tests do not substitute for a visual compositor/GPU smoke test.
