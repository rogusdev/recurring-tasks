use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tracing::{debug, warn};

use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio::{select, signal, spawn};

#[cfg(feature = "instant")]
use std::time::Instant;

#[cfg(feature = "instant")]
type RunTimer = Instant;

#[cfg(feature = "system")]
type RunTimer = SystemTime;

// Instant has no concept of time, so to get starting millis, we still need SystemTime, but just once at the start
fn now_since_epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Y2k happened?")
        .as_millis()
}

#[cfg(feature = "instant")]
fn run_timer_now() -> RunTimer {
    Instant::now()
}

#[cfg(feature = "instant")]
fn duration_since(now: RunTimer, old: RunTimer) -> Duration {
    now - old
}

#[cfg(feature = "system")]
fn run_timer_now() -> RunTimer {
    SystemTime::now()
}

#[cfg(feature = "system")]
fn duration_since(now: RunTimer, old: RunTimer) -> Duration {
    now.duration_since(old).expect("Old before now?")
}

/// Trait for tasks that can be run asynchronously, with the Task Manager
#[async_trait::async_trait]
pub trait AsyncTask: Send + Sync {
    /// The actual async task / work
    async fn run(&self) -> Result<(), String>;
    /// A name for logging
    fn name(&self) -> &str;
    /// The period/interval for this task to run on
    fn interval(&self) -> Duration;
}

/// Holds a single user task, when it started (if running), and when it should next run
struct ManagedTask {
    task: Arc<dyn AsyncTask>,
    started_at: Option<RunTimer>,
    next_run: RunTimer,
}

impl ManagedTask {
    fn new(task: Arc<dyn AsyncTask>) -> Self {
        Self {
            task,
            started_at: None,
            next_run: run_timer_now(),
        }
    }

    fn started_at(&self) -> Option<RunTimer> {
        self.started_at
    }

    fn start(&mut self) {
        self.started_at = Some(run_timer_now());
    }

    fn stop(&mut self) {
        self.started_at = None;
    }
}

/// Task manager that schedules and runs tasks
#[derive(Clone)]
pub struct TaskManager {
    tasks: Arc<Mutex<Vec<Arc<Mutex<ManagedTask>>>>>,
    /// How often should the manager check for tasks to run
    scheduler_tick: Duration,
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new(500)
    }
}

impl TaskManager {
    pub fn new(millis: u64) -> Self {
        TaskManager {
            tasks: Arc::new(Mutex::new(Vec::new())),
            scheduler_tick: Duration::from_millis(millis),
        }
    }

    pub async fn add<T>(&self, task: T)
    where
        T: AsyncTask + 'static,
    {
        let mut tasks = self.tasks.lock().await;

        let managed = ManagedTask::new(Arc::new(task));
        tasks.push(Arc::new(Mutex::new(managed)));
    }

    pub async fn run(&self) {
        debug!(
            "Initializing Recurring Tasks Manager using {}",
            if cfg!(feature = "instant") {
                "Instant"
            } else if cfg!(feature = "system") {
                "SystemTime"
            } else {
                "UNKNOWN"
            }
        );

        for managed_task in self.tasks.lock().await.iter() {
            let mut managed = managed_task.lock().await;

            let initial_delay = calculate_initial_delay(managed.task.interval());

            debug!(
                "Starting task {} in {} ms",
                managed.task.name(),
                initial_delay.as_millis(),
            );

            managed.next_run = run_timer_now() + initial_delay;
        }

        let tasks = self.tasks.clone();
        loop {
            let tasks = tasks.lock().await;
            for managed_task in tasks.iter() {
                let mut managed = managed_task.lock().await;
                let task_name = managed.task.name().to_owned();

                let now = run_timer_now();
                let prev_run = managed.next_run;
                if now >= prev_run {
                    // if it is already started, warn and skip
                    if let Some(started_at) = managed.started_at() {
                        debug!(
                            "Skipping run for task {task_name} (previous run from {:?} not finished)",
                            started_at
                        );
                    } else {
                        // Otherwise, mark it as running now, and schedule next run
                        managed.start();
                        let interval = managed.task.interval();
                        let next_run = prev_run + interval;
                        // check if we are falling too far behind on the schedule
                        managed.next_run = if next_run >= now {
                            next_run
                        } else {
                            let diff = duration_since(now, next_run);
                            warn!(
                                "Falling behind schedule on {task_name} by {} ms",
                                diff.as_millis()
                            );
                            now + interval
                        };

                        let managed_task = managed_task.clone();
                        spawn(async move {
                            debug!("Running task {task_name}");
                            if let Err(e) = managed_task.lock().await.task.run().await {
                                warn!("Error in task {task_name}: {e}");
                            }
                            managed_task.lock().await.stop();
                        });
                    }
                }
            }

            sleep(self.scheduler_tick).await;
        }
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
    let now_since_epoch_millis = now_since_epoch_millis();
    let interval_millis = interval.as_millis();

    // Calculate the next scheduled time
    // (millis are u128 and duration maxes at u64, so do u128 math before creating duration)
    let next_scheduled_time = ((now_since_epoch_millis / interval_millis) + 1) * interval_millis;
    let scheduled_from_now = next_scheduled_time - now_since_epoch_millis;
    Duration::from_millis(scheduled_from_now as u64)
}
