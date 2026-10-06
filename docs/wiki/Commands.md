# Commands and IPC

[Wiki](README.md) · [First session](Getting-Started.md)

`ghost-shell-daemon` runs the shell. `ghost-shell` sends commands to an already
running daemon in the same user session.

| Command | Action |
| --- | --- |
| `ghost-shell msg launcher toggle` | Open or close the application launcher |
| `ghost-shell msg finder toggle` | Open or close the file finder |
| `ghost-shell msg session lock` | Lock the session |

Inspect CLI syntax with:

```sh
ghost-shell --help
ghost-shell msg --help
```

The CLI connects to `$XDG_RUNTIME_DIR/ghost-shell-daemon`. `XDG_RUNTIME_DIR` must
be set, and both processes must share the same runtime directory. These are the
currently exposed commands; there are no reload, theme-switch, or widget-control
subcommands.

For global shortcuts, bind these commands in [Niri](Getting-Started.md#integrate-with-niri).
For lock prerequisites, see [lockscreen](Lockscreen.md).

Implementation: [CLI](../../crates/ghost-shell-cli/src/main.rs) and
[IPC protocol](../../crates/ghost-shell-ipc/src/protocol.rs).
