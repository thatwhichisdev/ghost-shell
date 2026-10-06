# Lockscreen

[Wiki](README.md) · [Wallpapers](Wallpapers.md) · [Commands](Commands.md)

Ghost Shell locks the Wayland session and uses the shared wallpaper source,
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

## Configuration

There is no `[lockscreen]` table. Configure its background through
`[wallpaper]` as described in [wallpapers](Wallpapers.md). The PAM service name
and username source are fixed in the implementation. Automatic idle locking and
suspend integration are not provided by the configuration schema; the CLI can
be called by your session tooling.

Implementation: [lock lifecycle](../../crates/ghost-shell-lockscreen/src/lockscreen.rs),
[authentication](../../crates/ghost-shell-lockscreen/src/auth.rs), and
[view](../../crates/ghost-shell-lockscreen/src/view.rs).
