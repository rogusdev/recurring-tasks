mod heartbeat;

use std::time::Duration;

use recurring_tasks::TaskManager;

use heartbeat::HeartbeatTask;

#[tokio::main]
async fn main() {
    let task_manager = TaskManager::default();

    task_manager
        .add(HeartbeatTask::new(Duration::from_secs(5)))
        .await;

    task_manager.run_with_signal().await;
    println!("Shutdown");
}
