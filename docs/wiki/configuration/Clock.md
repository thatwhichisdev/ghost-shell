# Clock

[Wiki](../README.md#configuration) · [Introduction](Introduction.md) ·
[Bar](Bar.md)

Status: **Implemented.**

Displays local time using Jiff's `strftime` formatting. The clock state and
configuration are shared by all bars.

## Setup and configuration

```toml
[clock]
format = "%H:%M"
```

`format` is a string and defaults to `"%H:%M"` (24-hour hours and minutes). For
a date alongside the time:

```toml
[clock]
format = "%Y-%m-%d %H:%M"
```

Choose one example for your configuration. The widget reads the system's local
time zone; there is no separate time-zone setting.

## Current limitations

The display updates on minute boundaries by default. Common second-bearing
directives (`%S`, `%T`, `%r`, `%X`, `%c`, `%s`, `%f`, `%N`) enable second
updates. There is no calendar popup or click action yet.

Implementation: [widget
crate](../../../crates/ghost-shell-widgets/ghost-shell-widget-clock/src/clock.rs).
