use std::time::Duration;

use tracing::info;

use deadpool_postgres::Pool;

use recurring_tasks::AsyncTask;

pub struct QueryTask {
    name: String,
    interval: Duration,
    pool: Pool,
}

impl QueryTask {
    pub fn new(interval: Duration, pool: Pool) -> Self {
        let name = format!("Query at {interval:?}");
        Self {
            name,
            interval,
            pool,
        }
    }
}

#[async_trait::async_trait]
impl AsyncTask for QueryTask {
    fn name(&self) -> &str {
        &self.name
    }

    fn interval(&self) -> Duration {
        self.interval
    }

    async fn run(&self) -> Result<(), String> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| format!("Db connection failed: {e}"))?;

        info!("Querying...");
        let rows: Vec<i32> = client
            .query("SELECT 1", &[])
            .await
            .map_err(|e| format!("Failed querying: {e}"))?
            .into_iter()
            .map(|row| row.get::<usize, i32>(0))
            .collect();
        info!("rows: {rows:?}");

        Ok(())
    }
}
