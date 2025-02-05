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
        .add(HeartbeatTask::new(Duration::from_millis(20000)))
        .await;

    task_manager
        .add(QueryTask::new(Duration::from_millis(50000), pool.clone()))
        .await;

    task_manager.run_with_signal().await;
    info!("Shutdown");
}
