# Ghost GPUI

Ghost's local Linux/Wayland UI framework, with Vulkan and OpenGL rendering.
The shell and its UI components use this framework directly.

See [ORIGIN.md](ORIGIN.md) for the source revision, module boundaries,
removed functionality, and verification commands.

Build the local framework:

```sh
cargo build -p ghost-shell-gpui -p ghost-shell-tokio
```

For a graphical smoke test inside a Wayland session:

```sh
cargo run -p ghost-shell-gpui --example hello_shell
```

Use `ghost-shell-gpui` as the public Cargo dependency:

```toml
[dependencies]
ghost-shell-gpui.workspace = true
```

Import it directly under its Rust crate name:

```rust,ignore
use ghost_shell_gpui::{Context, Render, Window, div, prelude::*};
```

Derives, actions, and test macros also use `ghost_shell_gpui`; no `gpui` alias
is needed. The runtime, Wayland backend, renderer, collections, scheduler,
sum tree, and refinement types are modules within this crate. Procedural macros
live in the sibling `ghost-shell-gpui-macros` crate and are re-exported here;
consumers do not need to depend on the macro crate directly.
