# Independent application lifecycle

Each application owns its executable, Waycrate runtime and Wayland surfaces.
There is no supervisor or shared runtime, and no dependencies between application
crates. Shared configuration does not imply shared process state.

| Application | Lifetime | Control |
| --- | --- | --- |
| Bar | Resident | IPC and Home Manager user service |
| Lockscreen | Resident | IPC and Home Manager user service; waits for unlock before stopping |
| Launcher | On demand | Direct Niri spawn; exits on Escape, closure or successful launch |
| Finder | On demand | Direct Niri spawn; exits on Escape or closure |
| Greeter | Boot login | greetd and separate system configuration |

Launcher and finder use process file locks to suppress duplicate invocations.
They expose no IPC endpoints. Their initial surfaces request the active output
from the compositor. The bar menu asks Niri to spawn the launcher.

Finder scans the resolved user home, never `/` by default. Its UI opens without
waiting for indexing and accepts typing while a worker initializes the index.
Enter during indexing queues a search for completion. Disk index persistence is
not implemented. Opening finder results remains a separate unfinished feature.

## Delivery checklist

- [x] Independent process ownership and shared TOML section loading.
- [x] Launcher/finder open on startup and exit instead of remaining resident.
- [x] Remove launcher/finder IPC, CLI control commands and Home Manager services.
- [x] Update Niri binding examples and bar launcher invocation.
- [x] Restrict finder indexing to the user home and allow input during startup.
- [ ] Validate on-demand startup latency and dismissal in a live Niri session.

See [architecture](wiki/Architecture.md), [commands](wiki/Commands.md), and
[installation migration](wiki/Installation.md#home-manager).
