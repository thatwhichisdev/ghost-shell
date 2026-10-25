use std::time::Duration;

use iced::{
    futures::{SinkExt, Stream},
    stream,
};
use sysinfo::System;
use tokio::time::MissedTickBehavior;

use crate::SystemInfo;

pub fn worker(interval: &Duration) -> impl Stream<Item = SystemInfo> + use<> {
    let interval = *interval;
    let interval = interval.max(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);

    stream::channel(1, async move |mut output| {
        let mut system = System::new();
        let mut ticker = tokio::time::interval(interval);

        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;

            system.refresh_cpu_usage();
            system.refresh_memory();

            let event = SystemInfo {
                cpu_usage: system.global_cpu_usage(),
                mem_usage: system.used_memory(),
                mem_total: system.total_memory(),
            };

            if output.send(event).await.is_err() {
                break;
            }
        }
    })
}
