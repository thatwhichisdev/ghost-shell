# Contributing

[Project overview](README.md) · [User documentation](wiki/README.md) ·
[Agent guide](../AGENTS.md)

Run the commands below from the repository root. For using the shell without
changing its code, start with [installation](wiki/Installation.md).

## Prerequisites

- Linux with a running Niri Wayland session for interactive testing.
- The Rust nightly pinned in [rust-toolchain.toml](../rust-toolchain.toml).
  Rustup selects it automatically inside the checkout.
- Native build tools, `pkg-config`, and Clang/libclang for bindgen.
- Development libraries for Fontconfig, FreeType, libxkbcommon, Vulkan, Wayland,
  Linux PAM, and PipeWire. See [nix/dev-shell.nix](../nix/dev-shell.nix) for the
  repository's dependency list. GPUI's upstream
  [Linux development guide](https://github.com/zed-industries/zed/blob/main/docs/src/development/linux.md)
  is additional background; this repository vendors its own GPUI fork.

Clone the repository:

```sh
git clone https://github.com/thatwhichisdev/ghost-shell
cd ghost-shell
```

With Nix and flakes enabled, enter the supplied development environment:

```sh
nix develop
```

It supplies the toolchain and native libraries and starts Nushell using the
repository's `.config/` files. These files configure development tools, not the
running shell.

## Build and run

```sh
cargo build
cargo run
```

The workspace's default member is `ghost-shell-daemon`, so these commands build
and run the daemon. Build the separate control CLI when testing IPC:

```sh
cargo build -p ghost-shell-cli
cargo run -p ghost-shell-cli -- msg launcher toggle
```

The daemon must already be running in the same user session. Configure it using
[getting started](wiki/Getting-Started.md) and the
[configuration reference](wiki/Configuration.md). A missing or invalid config
falls back to defaults, which contain no bars. There is no separate development
configuration file or command-line config override.

For release builds of both executables:

```sh
cargo build --release -p ghost-shell-daemon -p ghost-shell-cli
```

The Nix package builds both executables and wraps the daemon's runtime libraries:

```sh
nix build .#ghost-shell --print-build-logs
```

## Checks

Check formatting and run tests for the crates affected by a code change:

```sh
cargo fmt --all -- --check
cargo test -p ghost-shell-config
```

Replace the test package with the relevant crate. `cargo fmt --all` applies Rust
formatting; `nix fmt` applies Nix formatting and modifies files in the checkout.
The [CI workflow](../.github/workflows/ci.yml) checks Rust and Nix formatting and
builds the Nix package. Match those checks for changes affecting the build.

For UI changes, also test in Niri: startup, the affected surface or widget,
keyboard interaction, and relevant output changes. Unit tests alone do not
validate Wayland rendering or desktop services.

For documentation-only changes, check relative links, image paths, Markdown
formatting, and configuration examples against the current source. A full Rust
build is not needed unless the documentation change also changes code.

## Repository layout

| Path | Purpose |
| --- | --- |
| `crates/ghost-shell-daemon/` | Application entry point and service initialization |
| `crates/ghost-shell-cli/` | User-facing `ghost-shell` command |
| `crates/ghost-shell-config/` | Configuration schema, defaults, and loading |
| `crates/ghost-shell-widgets/` | Bar widgets |
| `crates/ghost-shell-components/` | Reusable UI components |
| Other `crates/ghost-shell-*/` directories | Surfaces, integrations, and shared services |
| `nix/` and `flake.nix` | Package, development environment, Home Manager module |
| `.config/` | Development tool configuration |
| `assets/` | Documentation screenshots |
| `crates/ghost-shell-assets/` | Assets embedded into the application |
| `LICENSE.md` | Project license |
| `docs/` | Project overview, development and agent guides |
| `docs/wiki/` | User guides and architecture documentation |

See [architecture](wiki/Architecture.md) for the runtime relationships and
[AGENTS.md](../AGENTS.md) for Rust, GPUI, and performance conventions.

## Documentation workflow

Keep the README focused on the project overview, screenshots, roadmap, and links.
Keep contributor setup and commands here. Put user-facing installation,
configuration, feature behavior, and troubleshooting in `docs/wiki/`.

The wiki uses plain Markdown with relative links, inspired by
[Niri's documentation structure](https://github.com/niri-wm/niri/wiki).
[Its index](wiki/README.md) is the entry point; no publishing pipeline is required
to browse it in the repository.

When adding a feature or widget:

1. Add a page linked from the wiki index or the widget index.
2. Describe its purpose, setup, interaction, configuration defaults, and limits.
3. Include a minimal working example only for supported settings. Explicitly say
   when there are no dedicated settings.
4. Link the implementation used to verify the description.
5. Update related guides and the roadmap when appropriate. Describe planned
   behavior separately from what currently works.

Use `../../assets/` for screenshots in top-level wiki pages and relative links
back to the relevant source. Widget pages follow the same structure under
`wiki/widgets/` and need one extra `../` to reach repository files.

## Vendored code and attribution

Consult the [project license](../LICENSE.md),
[GPUI origin notes](../crates/ghost-shell-gpui/ORIGIN.md),
[GPUI license](../crates/ghost-shell-gpui/LICENSE-APACHE),
[component upstream notes](../crates/ghost-shell-components/UPSTREAM.md), and
[Lucide license](../crates/ghost-shell-components/ghost-shell-component-icon/LICENSE-LUCIDE)
when working on vendored code or assets. Preserve their attribution notices.
