use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use iced::{
    futures::{SinkExt, Stream},
    stream,
};
use sysinfo::{Gpus, System};
use tokio::time::MissedTickBehavior;

use crate::{Error, SystemInfo, Update};

struct Sampler {
    system: System,
    gpus: Result<Gpus, Arc<Error>>,
}

impl Sampler {
    fn new() -> Result<Self, Error> {
        let mut system = System::new().map_err(|source| Error::System { source })?;
        // CPU utilization needs a baseline before the first timed sample.
        system.refresh_cpu_usage();
        Ok(Self {
            system,
            gpus: Gpus::new_with_refreshed_list()
                .map_err(|source| Arc::new(Error::Gpu { source })),
        })
    }

    fn sample(&mut self) -> SystemInfo {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        let (gpu_usage, gpu_error) = match &mut self.gpus {
            Ok(gpus) => {
                gpus.refresh(true);
                (
                    gpus.iter()
                        .filter_map(|gpu| gpu.usage())
                        .filter(|usage| usage.is_finite())
                        .map(|usage| usage.clamp(0.0, 100.0))
                        .reduce(f32::max),
                    None,
                )
            }
            Err(error) => (None, Some(Arc::clone(error))),
        };
        SystemInfo {
            sampled_at: Instant::now(),
            cpu_usage: (!self.system.cpus().is_empty())
                .then(|| self.system.global_cpu_usage()),
            gpu_usage,
            mem_usage: self.system.used_memory(),
            mem_total: self.system.total_memory(),
            gpu_error,
        }
    }
}

pub fn worker(interval: &Duration) -> impl Stream<Item = Update> + use<> {
    let interval = (*interval).max(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    stream::channel(1, async move |mut output| {
        let (requests, mut requested) =
            tokio::sync::mpsc::channel::<tokio::sync::oneshot::Sender<SystemInfo>>(1);
        // Gpus owns thread-local driver handles and is not Send. Keep it on
        // one blocking worker; dropping requests also releases that worker.
        let worker = tokio::task::spawn_blocking(move || -> Result<(), Error> {
            let mut sampler = Sampler::new()?;
            let baseline = Instant::now();
            while let Some(response) = requested.blocking_recv() {
                let mut sample = sampler.sample();
                if baseline.elapsed() < sysinfo::MINIMUM_CPU_UPDATE_INTERVAL {
                    sample.cpu_usage = None;
                }
                if response.send(sample).is_err() {
                    break;
                }
            }
            Ok(())
        });
        let mut ticker =
            tokio::time::interval_at(tokio::time::Instant::now() + interval, interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let (response, received) = tokio::sync::oneshot::channel();
            if requests.send(response).await.is_err() {
                break;
            }
            let Ok(sample) = received.await else {
                break;
            };
            if output.send(Ok(sample)).await.is_err() {
                return;
            }
        }
        drop(requests);
        let error = match worker.await {
            Ok(Ok(())) => return,
            Ok(Err(error)) => error,
            Err(source) => Error::Worker { source },
        };
        if output
            .send(Err(Arc::new(error)))
            .await
            .is_err()
        {
            return;
        }
    })
}
