# Commands and IPC

[Wiki](README.md) · [First session](Getting-Started.md)

`ghost-shell` controls the running bar and lockscreen daemons. Launcher and
finder are regular applications launched directly, without IPC servers.

| Command | Action |
| --- | --- |
| `ghost-shell-launcher` | Open the launcher; exit after a successful launch or Escape |
| `ghost-shell-finder` | Open the finder; exit on Escape or surface closure |
| `ghost-shell msg bar open` / `close` / `toggle` | Show or hide bars and their popups; wallpaper remains |
| `ghost-shell msg session lock` | Lock the session; acknowledge compositor acceptance |
| `ghost-shell msg shutdown bar` / `lockscreen` | Stop the daemon; an active lock waits for authentication |

Repeated launcher/finder invocations exit when another instance already holds
that application's process lock. They do not toggle or control the existing window.
Replace old `msg launcher` and `msg finder` bindings with the executable names.

Inspect CLI syntax with:

```sh
ghost-shell --help
ghost-shell msg --help
```

The CLI connects directly to `$XDG_RUNTIME_DIR/ghost-shell-{bar,lockscreen}`
according to the command. `XDG_RUNTIME_DIR` must be set and shared with the target.
Daemons are not started implicitly; a missing socket produces a connection
error. There are no reload or theme-switch subcommands.

UI `Handled` replies acknowledge accepted commands, not a completed render. Lock
replies wait for compositor acceptance. Closing launcher/finder surfaces terminates their processes. `ghost-shell msg shutdown lockscreen` acknowledges the
stop request immediately and defers exit until unlock when locked. Stopping one
application does not stop any other application.

For global shortcuts, bind these commands in [Niri](Getting-Started.md#integrate-with-niri).
For lock prerequisites, see [lockscreen](Lockscreen.md).

Implementation: [CLI](../../crates/ghost-shell-applications/ghost-shell-cli/src/main.rs) and
[IPC protocol](../../crates/ghost-shell-adapters/ghost-shell-ipc/src/protocol.rs).
