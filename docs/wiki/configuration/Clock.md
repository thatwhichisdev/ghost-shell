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

## Calendar

Click the clock to toggle a view-only calendar. It opens at the current local
month, aligned with the clock's right edge, 7px below the bar. It inherits the
bar's background opacity and blur; Escape and outside clicks dismiss it.

The Monday-first, six-week grid highlights today and dims days outside the
shown month. Month and year arrows browse the calendar; scrolling browses
months (up goes backward, down goes forward). Clicking a day does nothing:
there is no selected date, submit action, event creation, or system date change.
Reopening resets to the current month. Month and weekday labels are English.

The reusable `ghost-shell-calendar-components` crate follows the layout of
[iced_aw's date picker](https://github.com/iced-rs/iced_aw), using Jiff for date
arithmetic and ordinary iced widgets instead of the picker overlay.

## Update frequency

The display updates on minute boundaries by default. Common second-bearing
directives (`%S`, `%T`, `%r`, `%X`, `%c`, `%s`, `%f`, `%N`) enable second
updates.

Implementation: [widget
crate](../../../crates/ghost-shell-widgets/ghost-shell-widget-clock/src/clock.rs).
