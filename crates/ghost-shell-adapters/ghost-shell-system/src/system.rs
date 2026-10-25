use std::time::Duration;

use iced::Subscription;

#[cfg(feature = "sysinfo")]
mod sysinfo;

#[cfg(feature = "sysinfo")]
use self::sysinfo::worker;

/// System information such as cpu and memory usage
#[derive(Debug, Default)]
pub struct SystemInfo {
    cpu_usage: f32,
    mem_usage: u64,
    mem_total: u64,
}

/// Method that produces a subscription that produces `SystemInfo` based on give time interval
pub fn subscribe(interval: Duration) -> Subscription<SystemInfo> {
    Subscription::run_with(interval, worker)
}
