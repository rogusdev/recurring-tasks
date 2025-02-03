use std::time::{Duration, SystemTime};

use tracing::info;

use recurring_tasks::AsyncTask;

pub struct HeartbeatTask {
    name: String,
    interval: Duration,
    started: SystemTime,
}

impl HeartbeatTask {
    pub fn new(interval: Duration) -> Self {
        let name = format!("Heartbeat at {interval:?}");
        let started = SystemTime::now();

        Self {
            name,
            interval,
            started,
        }
    }
}

#[async_trait::async_trait]
impl AsyncTask for HeartbeatTask {
    fn name(&self) -> &str {
        &self.name
    }

    fn interval(&self) -> Duration {
        self.interval
    }

    async fn run(&self) -> Result<(), String> {
        let elapsed = self
            .started
            .elapsed()
            .map_err(|e| format!("Time went backwards: {e}"))?;

        info!("{}: {}", self.name, elapsed.as_secs() % 100);

        Ok(())
    }
}
