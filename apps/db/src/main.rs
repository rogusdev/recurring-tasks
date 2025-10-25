mod pool;

mod heartbeat;
mod query;

use std::time::Duration;

use tracing::info;

use recurring_tasks::TaskManager;

use pool::pg_pool;

use heartbeat::HeartbeatTask;
use query::QueryTask;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let pool = pg_pool().await.expect("Failed to get pg pool");

    let task_manager = TaskManager::new(200);

    task_manager
        .add(
            "Heartbeat every 20 s",
            Duration::from_millis(20000),
            HeartbeatTask::new(),
        )
        .await;

    task_manager
        .add_offset(
            "Query every minute at the bottom",
            Duration::from_secs(60),
            Duration::from_secs(30),
            QueryTask::new(pool.clone()),
        )
        .await;

    task_manager.run_with_signal().await;
    info!("Shutdown");
}
