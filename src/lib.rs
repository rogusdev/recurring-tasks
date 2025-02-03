use std::sync::Arc;
use std::time::{Duration, SystemTime};

use futures::future::join_all;

use tracing::{debug, warn};

use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};
use tokio::{select, signal, spawn};

/// Trait for tasks that can be run asynchronously, with the Task Manager
#[async_trait::async_trait]
pub trait AsyncTask: Send + Sync {
    async fn run(&self) -> Result<(), String>;
    fn name(&self) -> &str;
    fn interval(&self) -> Duration;
}

/// Holds a single user task and a record of when it started (if running)
struct ManagedTask {
    task: Arc<dyn AsyncTask>,
    started_at: Option<SystemTime>,
}

impl ManagedTask {
    pub fn started_at(&self) -> Option<SystemTime> {
        self.started_at
    }

    pub fn start(&mut self) {
        self.started_at = Some(SystemTime::now());
    }

    pub fn stop(&mut self) {
        self.started_at = None;
    }
}

/// Task manager that schedules and runs tasks
#[derive(Clone)]
pub struct TaskManager {
    tasks: Arc<Mutex<Vec<Arc<Mutex<ManagedTask>>>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        TaskManager {
            tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn add<T>(&self, task: T)
    where
        T: AsyncTask + 'static,
    {
        let mut tasks = self.tasks.lock().await;

        let managed = ManagedTask {
            task: Arc::new(task),
            started_at: None,
        };

        tasks.push(Arc::new(Mutex::new(managed)));
    }

    pub async fn run(&self) {
        debug!("Initializing Recurring Task Manager");
        let tasks = self.tasks.lock().await;
        let mut handles = Vec::new();

        // TODO: maybe just calculate time between each task and sleep until then?
        for managed_task in tasks.iter().cloned() {
            warn!("Spawning task");
            let handle = spawn(async move {
                let (task_name, task_interval) = {
                    let managed = managed_task.lock().await;
                    (managed.task.name().to_owned(), managed.task.interval())
                };

                // Calculate the initial delay to align with the desired schedule
                let initial_delay = calculate_initial_delay(task_interval);
                debug!("Sleep {:?} to start task {task_name}", initial_delay);
                sleep(initial_delay).await;

                // Start task loop after the initial delay
                warn!("Starting task {task_name}");
                loop {
                    warn!("Loop for task {task_name}");
                    let start = Instant::now();
                    let task_name = task_name.clone();
                    let task_name_2 = task_name.clone();

                    if {
                        warn!("Lock for task {task_name}");
                        let mut managed = managed_task.lock().await;

                        // if it is already started, warn and skip
                        if let Some(started_at) = managed.started_at() {
                            debug!(
                                "Skipping run for task {task_name} (previous run from {:?} not finished)",
                                started_at
                            );
                            false
                        } else {
                            // Otherwise, mark it as running now
                            managed.start();
                            true
                        }
                    } {
                        warn!("Running task {task_name}");
                        let managed_task = managed_task.clone();
                        spawn(async move {
                            if let Err(e) = managed_task.lock().await.task.run().await {
                                warn!("Error in task {task_name}: {e}");
                            }
                            managed_task.lock().await.stop();
                        });
                    }

                    let run_time = Instant::now() - start;
                    let remaining = task_interval - run_time;
                    warn!(
                        "Sleeping {} after {} for task {task_name_2}",
                        remaining.as_millis(),
                        run_time.as_millis()
                    );
                    sleep(remaining).await;
                    warn!("Awake for task {task_name_2}");
                }
            });

            handles.push(handle);
        }

        // Now we wait for all tasks (which in theory never finish)
        join_all(handles).await;
    }

    pub async fn run_with_signal(&self) {
        let manager = self.clone();

        let run_handle = spawn(async move {
            manager.run().await;
        });

        select! {
            _ = signal::ctrl_c() => {
                warn!("Ctrl+C received, shutting down recurring tasks...");
            }
            _ = run_handle => {}
        }
    }
}

/// Calculates the initial delay to align with the next scheduled time
fn calculate_initial_delay(interval: Duration) -> Duration {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let interval_secs = interval.as_secs();

    // Calculate the next scheduled time
    let next_scheduled_time = ((now.as_secs() / interval_secs) + 1) * interval_secs;
    let next_scheduled_duration = Duration::from_secs(next_scheduled_time);

    // Calculate the initial delay
    next_scheduled_duration - now
}
