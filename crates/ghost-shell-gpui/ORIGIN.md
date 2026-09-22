# GPUI extraction

Imported from Zed commit `916fc2b8cb3a815cbef4a3b40e13081be72036b6`
(local checkout: zed-industries/zed). Original framework and helper code is
Apache-2.0; see LICENSE-APACHE. Keep this provenance when making subsequent changes.

## Crate boundaries

```text
ghost-shell-gpui (runtime, Linux/Wayland backend, WGPU renderer)
├── src/collections.rs + collections/
├── src/scheduler.rs + scheduler/
├── src/sum_tree.rs + sum_tree/
├── src/refineable.rs
├── src/renderer.rs + renderer/
└── ghost-shell-gpui-macros (sibling proc-macro crate, including Refineable)

ghost-shell-tokio ──> ghost-shell-gpui
ghost-shell-util (shared helpers)
```

The library root is src/gpui.rs. The runtime and renderer share types directly,
without a facade or a separate core crate. Rust requires procedural macros to
be compiled in a separate proc-macro crate; their public exports remain available
through ghost_shell_gpui. SharedString is also a runtime module.

The original test_assets and integration tests directories were removed during
consolidation. Unit tests that required the deleted font and image fixtures were
removed as well. Remaining unit tests stay beside their implementation modules;
the public-crate macro smoke test lives in the hello_shell example.

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
cargo build -p ghost-shell-gpui -p ghost-shell-tokio
cargo test -p ghost-shell-gpui --lib
cargo test -p ghost-shell-gpui --example hello_shell --features test-support
cargo check -p ghost-shell-gpui -p ghost-shell-tokio --all-targets --all-features
cargo run -p ghost-shell-gpui --example hello_shell
```

The last command needs a running Wayland compositor and a working GPU driver.
The unit tests do not substitute for a visual compositor/GPU smoke test.
