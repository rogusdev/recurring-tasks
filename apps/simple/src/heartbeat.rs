use recurring_tasks::{AsyncTask, CancellationToken};

pub struct HeartbeatTask;

#[async_trait::async_trait]
impl AsyncTask for HeartbeatTask {
    async fn run(&self, _cancel: CancellationToken) -> Result<(), String> {
        println!("Heartbeat");
        Ok(())
    }
}
