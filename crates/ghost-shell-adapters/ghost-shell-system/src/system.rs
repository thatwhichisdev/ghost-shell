use std::time::Instant;

#[cfg(feature = "sysinfo")]
mod sysinfo;

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub sampled_at: Instant,
    pub cpu_usage: Option<f32>,
    /// Highest reported utilization among the available GPUs.
    pub gpu_usage: Option<f32>,
    pub mem_usage: u64,
    pub mem_total: u64,
    pub gpu_error: Option<std::sync::Arc<Error>>,
}

impl SystemInfo {
    pub fn memory_usage(&self) -> Option<f32> {
        (self.mem_total > 0).then(|| {
            (self.mem_usage as f64 / self.mem_total as f64 * 100.0).clamp(0.0, 100.0)
                as f32
        })
    }
}

#[derive(Debug, snafu::Snafu)]
pub enum Error {
    #[cfg(feature = "sysinfo")]
    #[snafu(display("Could not initialize system monitoring"))]
    System { source: ::sysinfo::Error },
    #[cfg(feature = "sysinfo")]
    #[snafu(display("Could not initialize GPU monitoring"))]
    Gpu { source: ::sysinfo::Error },
    #[snafu(display("System monitoring worker stopped"))]
    Worker { source: tokio::task::JoinError },
}

pub type Update = Result<SystemInfo, std::sync::Arc<Error>>;

#[cfg(feature = "sysinfo")]
pub fn subscribe(interval: std::time::Duration) -> iced::Subscription<Update> {
    iced::Subscription::run_with(interval, self::sysinfo::worker)
}
