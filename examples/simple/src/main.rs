mod heartbeat;

use std::time::Duration;

use recurring_tasks::TaskManager;

use heartbeat::HeartbeatTask;

#[tokio::main]
async fn main() {
    let task_manager = TaskManager::default();

    task_manager
        .add(
            "Heartbeat at 5 sec",
            Duration::from_secs(5),
            HeartbeatTask {},
        )
        .await;

    task_manager.run_with_signal().await;
    println!("Shutdown");
}
