# Contributing

[Project overview](../README.md) · [User documentation](wiki/README.md) · [Agent
guide](../AGENTS.md)

Run the commands below from the repository root. For using the shell without
changing its code, start with
[installation](wiki/getting-started/Installation.md).

## Prerequisites

- Linux with a running Niri Wayland session for interactive testing.
- The Rust nightly pinned in [rust-toolchain.toml](../rust-toolchain.toml).
  Rustup selects it automatically inside the checkout.
- Native build tools, `pkg-config`, and Clang/libclang for bindgen.
- Development libraries for Fontconfig, FreeType, libxkbcommon, Vulkan, Wayland,
  Linux PAM, and PipeWire. See [nix/dev-shell.nix](../nix/dev-shell.nix) for the
  repository's dependency list. The GUI uses iced and the pinned Waycrate
  runtime.

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
cargo run --bin ghost-shell-bar
```

The default workspace members are bar, launcher, finder and lockscreen.
`cargo build` builds these desktop applications. Select one by binary name:

```sh
cargo run --bin ghost-shell-launcher
cargo run --bin ghost-shell-finder
cargo run --bin ghost-shell-lockscreen
```

`--bin` takes a target name, not a source directory. Bare `cargo run` requires a
binary selection because there are now multiple defaults. Package selection with
`cargo run -p ghost-shell-launcher` also works. Use `cargo build --workspace` to
build all six executables. Enable the bar and lockscreen Home Manager services,
or run those daemons directly. Run launcher and finder on demand; they open
immediately and exit on dismissal. The boot greeter runs through greetd; see the
[greeter guide](wiki/configuration/Greeter.md). Use the CLI only for daemon IPC:

```sh
cargo build -p ghost-shell-cli
cargo run -p ghost-shell-launcher
cargo run -p ghost-shell-finder
cargo run -p ghost-shell-cli -- msg bar toggle
```

The target daemon must already be running in the same user session. Configure it
using [getting started](wiki/getting-started/Installation.md) and the
[configuration reference](wiki/configuration/Introduction.md). A missing or
invalid config falls back to defaults, which show a bar on each connected
output. There is no separate development configuration file or command-line
config override.

For release builds of all six executables:

```sh
cargo build --workspace --release
```

The Nix package builds all six executables and wraps the GUI runtime libraries:

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
The [CI workflow](../.github/workflows/ci.yml) checks Rust and Nix formatting
and builds the Nix package. Match those checks for changes affecting the build.

For UI changes, also test in Niri: startup, the affected surface or widget,
keyboard interaction, and relevant output changes. Unit tests alone do not
validate Wayland rendering or desktop services.

For documentation-only changes, check relative links, image paths, Markdown
formatting, and configuration examples against the current source. A full Rust
build is not needed unless the documentation change also changes code.

## Repository layout

  | Path                                               | Purpose                                                                           |
  | -------------------------------------------------- | --------------------------------------------------------------------------------- |
  | `crates/ghost-shell-applications/ghost-shell-cli/` | User-facing `ghost-shell` command                                                 |
  | `crates/ghost-shell-core/`                         | Embedded assets, configuration and theme                                          |
  | `crates/ghost-shell-applications/`                 | Independent applications: bar, CLI, finder, launcher, lockscreen and boot greeter |
  | `crates/ghost-shell-components/`                   | Reusable UI/rendering components, including wallpaper                             |
  | `crates/ghost-shell-adapters/`                     | Audio, D-Bus, IPC, Niri and system integrations                                   |
  | `crates/ghost-shell-widgets/`                      | Separate iced crate for each implemented bar widget                               |
  | `nix/` and `flake.nix`                             | Package, development environment, Home Manager module                             |
  | `.config/`                                         | Development tool configuration                                                    |
  | `assets/`                                          | Documentation screenshots                                                         |
  | `LICENSE.md`                                       | Project license                                                                   |
  | `docs/`                                            | Project overview, development and agent guides                                    |
  | `docs/wiki/`                                       | User guides and architecture documentation                                        |

See [architecture](wiki/development/Design-Principles-and-Architecture.md) for
the runtime relationships and [AGENTS.md](../AGENTS.md) for Rust, iced, and
performance conventions.

## Dependency declarations

Keep `[workspace.dependencies]` as a flat catalog of shared versions, internal
crate paths, pinned Git revisions, and default-feature policy. Select dependency
features in each consuming crate's manifest; do not put feature lists in the
root. Dependencies used by only one crate may remain local. Declare test-only
features under `[dev-dependencies]` and optional capabilities under the owning
package's `[features]` (a virtual workspace has no shared `[features]` table).

Each crate using iced explicitly selects `tokio`, `wayland` and `wgpu` so it can
build in isolation: iced requires an executor, platform backend and renderer.
Add `svg`, `image-without-codecs`, or other capabilities where used. The
launcher and tray explicitly enable raster codecs through an `image` dependency
because iced decodes their icons using that crate. Wallpaper requests its
supported GIF, PNG and JPEG codecs. Keep default features disabled centrally for
iced, iced_aw, Waycrate, image and zbus. Tray and audio select iced_aw's `menu`
feature. Each widget owns a popup adapter that opens an inert menu root and
measures iced_aw's overlay layout to resize its Wayland surface. These layout
contracts are covered by headless interaction tests, so iced_aw is pinned
exactly to 0.14.1 and both adapters must be checked when upgrading. This release
uses iced 0.14; upstream main targets iced development APIs and must not be
substituted without checking compatibility.

Check affected members separately, including test targets, before checking the
whole workspace. Unified workspace features can otherwise hide an incomplete
member declaration:

```sh
cargo check -p ghost-shell-niri --all-targets
cargo check --workspace --all-targets
cargo tree -e features -i image
```

Feature lists are additive; a member cannot subtract features enabled elsewhere
in the same build. Preserve the pinned Waycrate revision and existing version
requirements when reorganizing manifests.

## Documentation workflow

Keep the README focused on the project overview, screenshots, roadmap, and
links. Keep contributor setup and commands here. Put user-facing installation,
configuration, feature behavior, and troubleshooting in `docs/wiki/`.

The wiki uses plain Markdown with relative links, inspired by [Niri's
documentation structure](https://github.com/niri-wm/niri/wiki). [Its
index](wiki/README.md) is the entry point; no publishing pipeline is required to
browse it in the repository.

When adding a feature or widget:

1. Update the relevant Configuration page and its wiki index entry.
2. Describe its purpose, setup, interaction, configuration defaults, and limits.
3. Include a minimal working example only for supported settings. Explicitly say
   when there are no dedicated settings.
4. Link the implementation used to verify the description.
5. Update related guides and the roadmap when appropriate. Describe planned
   behavior separately from what currently works.

The wiki has three sections: `getting-started/` for NixOS installation and first
use, `configuration/` for shared rules and application/widget guides, and
`development/` for design principles, architecture, and UI/UX. Fold supporting
material into the relevant guide instead of creating unrelated top-level pages.
Use `../../../assets/` for screenshots in section pages and relative links to
source. Keep the [design
principles](wiki/development/Design-Principles-and-Architecture.md), [UI/UX
rules](wiki/development/UI-UX.md), and [agent guide](../AGENTS.md) aligned.

## Runtime validation

Run `cargo test --workspace` for portable tests. On a host with a graphics
adapter, run `cargo test -p ghost-shell-wallpaper -- --ignored` to validate the
custom wgpu pipeline. D-Bus tests start private buses and require permission to
create Unix sockets. Test compositor operations in an isolated nested Niri
session.
