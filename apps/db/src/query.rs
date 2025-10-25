use tracing::info;

use deadpool_postgres::Pool;

use recurring_tasks::AsyncTask;

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
