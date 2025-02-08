use std::time::SystemTime;

use tracing::info;

use recurring_tasks::AsyncTask;

pub struct HeartbeatTask {
    started: SystemTime,
}

impl HeartbeatTask {
    pub fn new() -> Self {
        let started = SystemTime::now();

        Self { started }
    }
}

#[async_trait::async_trait]
impl AsyncTask for HeartbeatTask {
    async fn run(&self) -> Result<(), String> {
        let elapsed = self
            .started
            .elapsed()
            .map_err(|e| format!("Time went backwards: {e}"))?;

        info!("Heartbeat: {}", elapsed.as_secs() % 100);

        Ok(())
    }
}
