use tracing::{debug, info};

use tokio::select;

use deadpool_postgres::Pool;

use recurring_tasks::{AsyncTask, CancellationToken};

pub struct QueryTask {
    pool: Pool,
}

impl QueryTask {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl AsyncTask for QueryTask {
    async fn run(&self, cancel: CancellationToken) -> Result<(), String> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| format!("Db connection failed: {e}"))?;

        info!("Querying...");
        select! {
            // OR: to test long running task: SELECT pg_sleep(50000)
            res = client.query("SELECT 1", &[]) => {
                let rows: Vec<i32> = res
                    .map_err(|e| format!("Failed querying: {e}"))?
                    .into_iter()
                    .map(|row| row.get::<usize, i32>(0))
                    .collect();
                info!("rows: {rows:?}");
            }
            _ = cancel.cancelled() => {
                debug!("Cancelled Query task");
            }
        }

        Ok(())
    }
}
