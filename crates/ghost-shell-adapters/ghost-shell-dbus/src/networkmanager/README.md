# NetworkManager bindings

Copied from [pop-os/dbus-settings-bindings/networkmanager](https://github.com/pop-os/dbus-settings-bindings/tree/85502fb865695a15261a80dea0eef38a8c23e349/networkmanager),
revision `85502fb865695a15261a80dea0eef38a8c23e349`.
The upstream Rust files retain their MPL-2.0 SPDX headers and are covered by
[LICENSE.md](LICENSE.md). The original authors retain their copyrights.

Local adaptations: module paths are rooted at `crate::networkmanager`,
`interface/mod.rs` becomes `interface.rs`, `lib.rs` becomes `networkmanager.rs`,
formatting follows this repository, and malformed IPv6 addresses and invalid
object paths return errors instead of panicking. Dependencies use the workspace
catalog and zbus's zvariant re-export.

`monitor.rs` is Ghost's MIT-licensed status monitor. It uses the copied proxy
interfaces to observe Ethernet and Wi-Fi without changing system settings.
