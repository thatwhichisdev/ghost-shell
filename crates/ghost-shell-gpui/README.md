# Ghost GPUI

Ghost's local Linux/Wayland UI framework, with Vulkan and OpenGL rendering.
The application still uses upstream GPUI until its separate migration.

See [ORIGIN.md](ORIGIN.md) for the source revision, internal crate boundaries,
removed functionality, and verification commands.

Build the local framework:

```sh
cargo build -p ghost-shell-gpui -p ghost-shell-wgpu -p ghost-shell-tokio
```

For a graphical smoke test inside a Wayland session:

```sh
cargo run -p ghost-shell-gpui --example hello_shell
```

Use `ghost-shell-gpui` as the public dependency, aliased to `gpui` when retaining
upstream-style imports and procedural macros. Do not mix its entities with the
old upstream GPUI or components that still depend on it.
