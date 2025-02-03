use std::time::Duration;

use recurring_tasks::AsyncTask;

pub struct HeartbeatTask {
    name: String,
    interval: Duration,
}

impl HeartbeatTask {
    pub fn new(interval: Duration) -> Self {
        let name = format!("Heartbeat at {} ms", interval.as_millis());

        Self { name, interval }
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
        println!("{}", self.name);

        Ok(())
    }
}
