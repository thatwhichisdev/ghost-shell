# Design Principles and Architecture

[Wiki](../README.md#development) · [UI/UX](UI-UX.md) ·
[Contributing](../../CONTRIBUTING.md)

This guide defines the design rules for implementing Ghost Shell. Keep
[AGENTS.md](../../../AGENTS.md) aligned with these principles and the [UI/UX
guide](UI-UX.md). Contributor setup and checks live in
[Contributing](../../CONTRIBUTING.md).

## Design principles

Ghost Shell provides desktop tools for Niri while delegating window management
and application spawning to the compositor. Keep features focused, ownership
clear, and behavior simple enough to reason about.

- Prioritize correctness and clarity. Use safe, idiomatic Rust 2024 and narrow
  public APIs. Isolate necessary unsafe code and document its invariants.
- Keep applications independent. Share libraries without introducing a
  supervisor, shared runtime, or dependencies between application crates.
- Minimize idle CPU, memory, background activity, and startup latency. Prefer
  event-driven updates and immediate input response.
- Propagate recoverable errors to callers and interactive failures to the UI.
  Avoid runtime panics and never silently discard errors.
- Expose configuration for implemented behavior. Describe roadmap work
  separately.

## Project structure

The workspace groups crates by responsibility:

  | Directory under `crates/`  | Responsibility                                                                                                      |
  | -------------------------- | ------------------------------------------------------------------------------------------------------------------- |
  | `ghost-shell-adapters`     | Libraries integrating with external applications, services and protocols, such as Niri, PipeWire, D-Bus and greetd. |
  | `ghost-shell-applications` | Independent application binaries: bar, CLI, launcher, finder, lockscreen and greeter.                               |
  | `ghost-shell-components`   | Custom UI and rendering components built on iced. Currently contains the wallpaper component.                       |
  | `ghost-shell-core`         | Libraries shared by applications: configuration, theme and assets.                                                  |
  | `ghost-shell-widgets`      | Bar widgets, each defined in its own crate.                                                                         |

Each GUI application owns its runtime, state and Wayland surfaces. Applications
must not depend on other application crates or share a runtime. Bar and
lockscreen are resident daemons; launcher and finder run on demand; greeter runs
under greetd. The CLI sends commands to the daemons through IPC.

## Applications and adapters

Adapters own external integration logic. Applications own UI state and decide
how to react to adapter events and results. Widgets consume application-owned
state and emit messages rather than opening duplicate integration clients.

- **Event streams:** expose iced `Subscription`, or streams using the `sipper` /
  `Straw` interfaces that applications can consume through subscriptions or
  `Task::run`. Tie stream lifetime to its consumer and propagate stream
  failures.
- **Adapter calls:** wrap operations in iced `Task` at the application boundary.
  Map results into application messages; propagate errors to the UI instead of
  silently discarding them.
- **Execution:** keep blocking I/O and expensive work off the UI thread. Prefer
  event-driven updates over polling and keep cancellation and ownership
  explicit.

Adapters may retain framework-independent APIs for consumers such as the CLI.
Their iced-facing interfaces follow the same stream and task rules.

## Runtime ownership

  | Application | Lifetime and responsibility                                                                               |
  | ----------- | --------------------------------------------------------------------------------------------------------- |
  | Bar         | Resident daemon; owns bars, widget popups, desktop wallpaper, and its IPC server                          |
  | CLI         | Short-lived client; sends commands to the bar or lockscreen                                               |
  | Launcher    | Opens immediately, discovers applications, launches through Niri, exits on dismissal or successful launch |
  | Finder      | Opens immediately, indexes and searches files, releases its index on exit                                 |
  | Lockscreen  | Resident daemon; owns session-lock surfaces, unlock authentication, and its IPC server                    |
  | Greeter     | Runs under greetd; owns login surfaces and delegates authentication and session start to greetd           |

Keep surface creation, closure, output reconciliation, and popup state with the
owning application. Only bar and lockscreen have user-session IPC and Home
Manager services. Launcher and finder use process locks to prevent duplicates.

Desktop applications independently read shared `config.toml` and deserialize
only relevant sections. Sharing configuration does not imply sharing runtime
state. The greeter reads a separate system-owned file available before login.
See [configuration
discovery](../configuration/Introduction.md#file-discovery-and-loading).

Lockscreen alone owns session locking and PAM unlock authentication. An active
lock must finish authentication and protocol unlock before honoring exit.
Greeter owns no session lock; greetd owns login authentication and session
creation. Preserve these boundaries when sharing rendering components.

## State and rendering flow

Adapter events update application-owned state. Views render that state and
widgets emit messages for user actions. Applications dispatch adapter work and
map results back into state and visible feedback. Avoid opening an integration
client per widget or per output when the data is shared.

Keep discovery, indexing, image decoding, and blocking I/O off the UI thread.
Render useful initial UI while work runs, retain current input, and release work
and resources when their owner closes. Update only affected state; avoid
full-state recomputation and unnecessary redraws.

### Live bar configuration

The config crate's optional `watch` feature exposes an owned, event-driven stream.
Directory watches survive atomic editor saves and Home Manager symlink replacement;
a 150ms debounce runs only after changes. Parsing runs off the UI thread. The bar
keeps its last valid file configuration separate from its effective preview.
Invalid reloads preserve both and expose feedback in the styling panel. A valid
change replaces the preview; resetting restores the file baseline. Other
applications retain their startup-loading lifecycle.

The bar owns surface updates and applies geometry/exclusive-zone changes in place.
The styling widget emits edits and renders controls; it does not own the runtime,
write config files, or create D-Bus clients. Preview values are process-local.
Nix export copies only general/bar/theme settings to the clipboard for the user
to merge into their source configuration. Shared theme changes in a preview do
not modify other running applications. Async icon and wallpaper refreshes guard
against stale completion. Home Manager uses live reload for the bar and retains
its restart trigger for the lockscreen.

### Wallpaper rendering

GIFs are decoded on a worker and stored as an initial RGBA frame plus compressed
sparse changes, including the transition back to the first frame. A versioned
cache is checked against the source path, size, and modification time. Each
surface has reusable playback buffers and a persistent GPU texture rendered
through an iced shader with cover scaling. Frame deadlines request redraws
locally; static images do not run an animation timer.

Very short GIF delays are clamped to 10 ms. After a long pause, playback resumes
with bounded catch-up while preserving the delta sequence. Images and animations
have memory and dimension limits; unsupported sources leave the fallback
visible.

## Implementation and dependencies

The crate directories group workspace members, not nested workspaces or umbrella
crates. Place binaries with their applications, including the CLI. Prefer
existing files unless a new logical component is needed. Use descriptive module
filenames rather than `mod.rs`, full words for variables, and comments
explaining non-obvious reasons. Avoid unnecessary cloning, allocation, dynamic
dispatch, synchronization, and intermediate collections. Use async execution
when it provides a concrete benefit.

Keep shared versions, paths, Git revisions, and default-feature policy in the
root workspace manifest. Select features in consumers and test-only features in
dev-dependencies. Check changed members with
`cargo check -p <package> --all-targets`, then check the workspace so feature
unification cannot hide missing requirements. See [dependency
declarations](../../CONTRIBUTING.md#dependency-declarations).

Use repository code and exact pinned dependency sources as the first reference.
Verify iced and Waycrate lifecycle and rendering APIs at the pinned revision;
use Niri's IPC definitions and Wayland specifications for protocol behavior. Do
not infer an API from a different release.

## Validation and documentation

Test state transitions, failures, and success with the relevant
[checks](../../CONTRIBUTING.md#checks). Validate surfaces, focus, output
changes, and session lifecycle in Niri; compilation does not establish
compositor behavior.

Profile meaningful hot paths in release builds when changing discovery, ranking,
compositor updates, widget propagation, or image processing. Avoid frequent
polling, unnecessary timers, repeated process creation, and persistent work in
applications intended to exit on dismissal.

Update configuration pages when fields or behavior change. Keep the README on
overview and roadmap, the wiki on user and design guidance, and AGENTS.md on
actionable instructions linked to these guides.
