mod heartbeat;

use std::time::Duration;

use recurring_tasks::TaskManager;

use heartbeat::HeartbeatTask;

#[tokio::main]
async fn main() {
    let mut task_manager = TaskManager::new();

    task_manager.add(
        "Heartbeat at 5 sec",
        Duration::from_secs(5),
        HeartbeatTask {},
    );

    task_manager.run_with_signal().await;
    println!("Shutdown");
}
