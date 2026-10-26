# Architecture

[Wiki](README.md)

## Project structure

The workspace groups crates by responsibility:

| Directory under `crates/` | Responsibility |
| --- | --- |
| `ghost-shell-adapters` | Libraries integrating with external applications, services and protocols, such as Niri, PipeWire, D-Bus and greetd. |
| `ghost-shell-applications` | Independent application binaries: bar, CLI, launcher, finder, lockscreen and greeter. |
| `ghost-shell-components` | Custom UI and rendering components built on iced. Currently contains the wallpaper component. |
| `ghost-shell-core` | Libraries shared by applications: configuration, theme and assets. |
| `ghost-shell-widgets` | Bar widgets, each defined in its own crate. |

Each GUI application owns its runtime, state and Wayland surfaces. Applications
must not depend on other application crates or share a runtime. Bar and lockscreen
are resident daemons; launcher and finder run on demand; greeter runs under greetd.
The CLI sends commands to the daemons through IPC.

## Applications and adapters

Adapters own external integration logic. Applications own UI state and decide how
to react to adapter events and results. Widgets consume application-owned state
and emit messages rather than opening duplicate integration clients.

- **Event streams:** expose iced `Subscription`, or streams using the `sipper` /
  `Straw` interfaces that applications can consume through subscriptions or
  `Task::run`. Tie stream lifetime to its consumer and propagate stream failures.
- **Adapter calls:** wrap operations in iced `Task` at the application boundary.
  Map results into application messages; propagate errors to the UI instead of
  silently discarding them.
- **Execution:** keep blocking I/O and expensive work off the UI thread. Prefer
  event-driven updates over polling and keep cancellation and ownership explicit.

Adapters may retain framework-independent APIs for consumers such as the CLI.
Their iced-facing interfaces follow the same stream and task rules.
