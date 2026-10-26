# Lockscreen

[Wiki](README.md) · [Wallpapers](Wallpapers.md) · [Commands](Commands.md)

Ghost Shell locks the Wayland session and uses its own wallpaper source,
including animated GIFs, behind its password input.

```sh
ghost-shell msg session lock
```

## Authentication setup

The current implementation authenticates the account named by `USER` through
the PAM service `ghost-shell`, then performs PAM account validation. Your system
must provide a suitable policy for that service, normally through
`/etc/pam.d/ghost-shell`. The package and Home Manager module do not create it.
Use your distribution's PAM configuration mechanism and authentication policy;
there is no distribution-independent PAM file supplied by this project.

Complete and test authentication setup before relying on the lockscreen in your
session. Ghost Shell remains early-alpha software.

## Diagnosing unlock failures

Failures log the PAM service, selected username, stage (`setup`, `authenticate`,
or `account`) and PAM error code without recording passwords. Read terminal
output when running directly, or use
`journalctl --user -u ghost-shell-lockscreen.service -b` for the standalone service.
Account validation can reject an unlock even after password authentication
succeeds. Do not assume that every failure means the password was incorrect.

If logs instead identify `ghost-shell-daemon` or `ghost-shell.service`, an older
installation is still running. Update the installed package and user services;
changes to the standalone lockscreen do not change that older process.

## Configuration

Use `~/.config/ghost-shell/config.toml`. There is no `[lockscreen]` table.
The lockscreen shares the desktop wallpaper, general settings and theme.
Configure the background through
`[wallpaper]` as described in [wallpapers](Wallpapers.md). The PAM service name
and username source are fixed in the implementation. Automatic idle locking and
suspend integration are not provided by the configuration schema; the CLI can
be called by your session tooling.

## Stopping the service

SIGTERM, SIGINT and `ghost-shell msg shutdown lockscreen` request exit. While a
lock is pending or active, exit waits for authentication and protocol unlock.
Home Manager's lockscreen service has no forced stop timeout and does not restart
after a crash. `systemctl --user restart ghost-shell-lockscreen` may therefore
wait until you unlock. Other applications can stop or restart independently.

Implementation: [lock lifecycle](../../crates/ghost-shell-applications/ghost-shell-lockscreen/src/app.rs),
[authentication](../../crates/ghost-shell-applications/ghost-shell-lockscreen/src/auth.rs), and
[view](../../crates/ghost-shell-applications/ghost-shell-lockscreen/src/lib.rs).
