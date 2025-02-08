use recurring_tasks::AsyncTask;

pub struct HeartbeatTask;

#[async_trait::async_trait]
impl AsyncTask for HeartbeatTask {
    async fn run(&self) -> Result<(), String> {
        println!("Heartbeat");
        Ok(())
    }
}
