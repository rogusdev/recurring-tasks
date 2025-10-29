//! Scheduled async tasks / jobs manager to run forever, but not run jobs if already/still running
//!
//! Simple example:
//! ```
//! use std::time::{Duration, SystemTime};
//!
//! use tracing::info;
//!
//! use recurring_tasks::{AsyncTask, TaskManager, CancellationToken};
//!
//! pub struct HeartbeatTask;
//!
//! #[async_trait::async_trait]
//! impl AsyncTask for HeartbeatTask {
//!     async fn run(&self, _cancel: CancellationToken) -> Result<(), String> {
//!         info!("Heartbeat");
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let task_manager = TaskManager::default();
//!
//!     // run a heartbeat task every 5 seconds
//!     task_manager
//!         .add("Heartbeat", Duration::from_secs(5), HeartbeatTask {})
//!         .await;
//!
//!     // this will run until ctl-c! not suitable for a cargo test example ;)
//!     //task_manager.run_with_signal().await;
//!     println!("Shutdown");
//! }
//! ```
//!
//! For a fancier example, see the repo: [db query task](https://github.com/rogusdev/recurring-tasks/blob/main/apps/db/src/main.rs)
//!
//! This crate is intended to be very direct and specific. For a more elaborate scheduling crate,
//! using crontab syntax, consider [tokio-cron-scheduler](https://github.com/mvniekerk/tokio-cron-scheduler).
//! There are also a variety of additional alternatives out there, each with different priorities.

use std::sync::Arc;
use std::time::Duration;

use tracing::{debug, error, warn};

use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};
use tokio::{select, spawn};
pub use tokio_util::sync::CancellationToken;

#[cfg(test)]
use mock_instant::global::SystemTime;
#[cfg(not(test))]
use std::time::SystemTime;

#[cfg(all(feature = "instant", test))]
use mock_instant::global::Instant;
#[cfg(all(feature = "instant", not(test)))]
use std::time::Instant;

#[cfg(feature = "instant")]
type RunTimer = Instant;

#[cfg(feature = "system")]
type RunTimer = SystemTime;

const DEFAULT_TICK_MILLIS: u64 = 500;

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
    /// The actual async task / work / job
    ///
    /// * `cancel` - Tokio CancellationToken (re-exported by this crate) to stop a run
    /// See query demo app for one example usage to interrupt async run operations.
    /// This is a child token that can only cancel this one task run, not the entire loop.
    async fn run(&self, cancel: CancellationToken) -> Result<(), String>;
}

/// Holds a single user task, when it started (if running), and when it should next run
struct ManagedTask {
    name: String,
    interval: Duration,
    offset: Duration,

    task: Arc<dyn AsyncTask>,
    started_at: Option<RunTimer>,
    next_run: RunTimer,
}

impl ManagedTask {
    fn new(name: String, interval: Duration, offset: Duration, task: Arc<dyn AsyncTask>) -> Self {
        Self {
            name,
            interval,
            offset,
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

/// Task manager that schedules and runs tasks on schedule, indefinitely
#[derive(Clone)]
pub struct TaskManager {
    tasks: Arc<Mutex<Vec<Arc<Mutex<ManagedTask>>>>>,
    /// How often should the manager check for tasks to run
    scheduler_tick: Duration,
}

impl Default for TaskManager {
    /// Defaults to 500 ms for checking for tasks to run
    fn default() -> Self {
        Self::new(DEFAULT_TICK_MILLIS)
    }
}

impl TaskManager {
    /// Specify the tick ms for frequency/interval of checking for tasks to run
    /// Also consider `::default()` for a sensible default for tasks on intervals of seconds and above
    pub fn new(millis: u64) -> Self {
        TaskManager {
            tasks: Arc::new(Mutex::new(Vec::new())),
            scheduler_tick: Duration::from_millis(millis),
        }
    }

    /// Add a task to be run periodically on an interval, without an offset
    ///
    /// * `name` - Unique, descriptive name for debug + error logging
    /// * `interval` - The period / frequency at which this task will run
    /// * `task` - The actual task / job / work that will be run on the interval
    ///
    /// To explain interval, consider 3 examples:
    /// - Interval 30 seconds == task will run every half minute (00:00:30, 00:01:00, 00:01:30...)
    /// - Interval  3,600 seconds (60 minutes) == task will run at the top of the hour (02:00:00, 03:00:00, 04:00:00...)
    /// - Interval 86,400 seconds (1 day / 24 hours) == task will run at midnight every day (00:00:00)
    ///
    /// This system runs on time passing only (with default features) and should be unaffected by any daylight savings times,
    /// although the starting runs of all tasks do initialize based on current system clock, whatever timezone that is
    pub async fn add<T>(&self, name: &str, interval: Duration, task: T)
    where
        T: AsyncTask + 'static,
    {
        self.add_offset(name, interval, Duration::ZERO, task).await
    }

    /// Add a task to be run periodically on an interval, with an offset
    ///
    /// * `name` - Unique, descriptive name for debug + error logging
    /// * `interval` - The period / frequency at which this task will run
    /// * `offset` - The offset at which this interval will begin
    /// * `task` - The actual task / job / work that will be run on the interval
    ///
    /// To explain offset, consider 3 examples, all with an interval of 60 minutes (1 hour):
    /// - Offset not provided (0) == task will run at the top of the hour (2:00, 3:00, 4:00...)
    /// - Offset 30 min == task will run at half past the hour every hour (2:30, 3:30, 4:30...)
    /// - Offset 15 min == task will run at quarter past the hour every hour (2:15, 3:15, 4:15...)
    pub async fn add_offset<T>(&self, name: &str, interval: Duration, offset: Duration, task: T)
    where
        T: AsyncTask + 'static,
    {
        if interval == Duration::ZERO {
            panic!("Interval must be nonzero!");
        }
        if offset >= interval {
            panic!("Offset must be strictly less than interval!");
        }

        let managed = ManagedTask::new(name.to_owned(), interval, offset, Arc::new(task));
        let mut tasks = self.tasks.lock().await;
        tasks.push(Arc::new(Mutex::new(managed)));
    }

    /// Run the tasks in the task manager on schedule until the process dies
    pub async fn run(&self) {
        self.run_with_cancel(CancellationToken::new()).await
    }

    /// Run the tasks in the task manager on schedule until cancelled
    ///
    /// * `cancel` - token to cancel() to stop the manager/loop
    pub async fn run_with_cancel(&self, cancel: CancellationToken) {
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

            let initial_delay = calculate_initial_delay(managed.interval, managed.offset);

            debug!(
                "Starting task {} in {} ms",
                managed.name,
                initial_delay.as_millis(),
            );

            managed.next_run = run_timer_now() + initial_delay;
        }

        let tasks = self.tasks.clone();
        loop {
            let tasks = tasks.lock().await;
            for managed_task in tasks.iter() {
                let mut managed = managed_task.lock().await;
                let task_name = managed.name.clone();

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
                        let interval = managed.interval;
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
                        let cancel = cancel.child_token();
                        spawn(async move {
                            debug!("Running task {task_name}");
                            if let Err(e) = managed_task.lock().await.task.run(cancel).await {
                                warn!("Error in task {task_name}: {e}");
                            }
                            managed_task.lock().await.stop();
                        });
                    }
                }
            }

            select! {
                _ = sleep(self.scheduler_tick) => {}
                _ = cancel.cancelled() => {
                    debug!("Cancelled Recurring Tasks Manager loop");
                    break;
                }
            }
        }
    }

    /// Run the tasks in the task manager on schedule until the process is interrupted
    pub async fn run_with_signal(self) {
        let cancel = CancellationToken::new();

        let mut handle = spawn({
            let cancel = cancel.child_token();
            async move {
                self.run_with_cancel(cancel).await;
            }
        });

        select! {
            res = &mut handle => {
                error!("Manager stopped unexpectedly: {res:?}")
            }
            _ = shutdown_signal() => {
                warn!("Shutdown signal received, stopping recurring tasks...");
                // tell manager tasks loop to stop
                cancel.cancel();
                // give tasks some time to finish gracefully
                match timeout(Duration::from_secs(20), &mut handle).await {
                    Ok(_) => debug!("Shutdown complete"),
                    Err(_) => {
                        warn!("Aborting tasks after timeout");
                        handle.abort();
                        // wait to finish abort -- process will be killed if this takes too long
                        let _ = handle.await;
                    }
                }
            }
        }
    }
}

async fn shutdown_signal() {
    let sigint = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let sigterm = async {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut s) = signal(SignalKind::terminate()) {
            s.recv().await;
        }
    };

    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    tokio::select! {
        _ = sigint  => {},
        _ = sigterm => {},
    }
}

/// Calculates the initial delay to align with the next scheduled time
///
/// NOTE: TaskManager verifies that intervals are all strictly greater than offsets
fn calculate_initial_delay(interval: Duration, offset: Duration) -> Duration {
    let now_since_epoch_millis = now_since_epoch_millis();
    let interval_millis = interval.as_millis();
    let offset_millis = offset.as_millis();

    // Calculate the next scheduled time
    // (millis are u128 and duration maxes at u64, so do u128 math before creating duration)
    let next_scheduled_time =
        (now_since_epoch_millis / interval_millis) * interval_millis + offset_millis;
    // check if offset puts this earlier or later
    let scheduled_from_now = if next_scheduled_time > now_since_epoch_millis {
        next_scheduled_time - now_since_epoch_millis
    } else {
        next_scheduled_time + interval_millis - now_since_epoch_millis
    };
    Duration::from_millis(scheduled_from_now as u64)
}

#[cfg(test)]
mod tests {
    use std::sync::Once;

    use mock_instant::global::MockClock;

    use super::*;

    static INIT: Once = Once::new();

    /// init_logging() called at start of any test fn while debugging with logs
    #[allow(unused)]
    pub fn init_logging() {
        use tracing_subscriber::{EnvFilter, fmt};

        INIT.call_once(|| {
            let filter =
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
            fmt().with_env_filter(filter).with_test_writer().init();
        });
    }

    #[derive(Clone)]
    pub struct TestTask {
        count: Arc<Mutex<usize>>,
    }

    impl TestTask {
        pub fn new() -> Self {
            Self {
                count: Arc::new(Mutex::new(0)),
            }
        }

        pub async fn count(&self) -> usize {
            *self.count.lock().await
        }
    }

    #[async_trait::async_trait]
    impl AsyncTask for TestTask {
        async fn run(&self, _cancel: CancellationToken) -> Result<(), String> {
            let mut count = self.count.lock().await;
            *count += 1;
            Ok(())
        }
    }

    #[test]
    fn half_offset() {
        let interval = Duration::from_secs(60);
        let offset = Duration::from_secs(30);

        MockClock::set_system_time(Duration::from_secs(0));
        let delay = calculate_initial_delay(interval, offset);
        assert_eq!(delay, offset, "0 is offset");

        MockClock::set_system_time(offset);
        let delay = calculate_initial_delay(interval, offset);
        assert_eq!(delay, interval, "offset is interval");

        let diff = Duration::from_secs(15);
        MockClock::set_system_time(offset - diff);
        let delay = calculate_initial_delay(interval, offset);
        assert_eq!(delay, diff, "less than offset is offset remainder");

        let diff = Duration::from_secs(15);
        MockClock::set_system_time(offset + diff);
        let delay = calculate_initial_delay(interval, offset);
        assert_eq!(
            delay,
            interval - diff,
            "more than offset is interval remainder"
        );
    }

    #[test]
    fn quarter_offset() {
        let interval = Duration::from_secs(60);
        let offset = Duration::from_secs(15);

        MockClock::set_system_time(Duration::from_secs(0));
        let delay = calculate_initial_delay(interval, offset);
        assert_eq!(delay, offset, "0 is offset");

        MockClock::set_system_time(offset);
        let delay = calculate_initial_delay(interval, offset);
        assert_eq!(delay, interval, "offset is interval");

        let diff = Duration::from_secs(5);
        MockClock::set_system_time(offset - diff);
        let delay = calculate_initial_delay(interval, offset);
        assert_eq!(delay, diff, "less than offset is offset remainder");

        let diff = Duration::from_secs(15);
        MockClock::set_system_time(offset + diff);
        let delay = calculate_initial_delay(interval, offset);
        assert_eq!(
            delay,
            interval - diff,
            "more than offset is interval remainder"
        );
    }

    #[tokio::test]
    #[should_panic(expected = "Interval must be nonzero!")]
    async fn interval_nonzero() {
        TaskManager::default()
            .add("Fails", Duration::from_secs(0), TestTask::new())
            .await;
    }

    #[tokio::test]
    #[should_panic(expected = "Offset must be strictly less than interval!")]
    async fn offset_match_interval() {
        TaskManager::default()
            .add_offset(
                "Fails",
                Duration::from_secs(10),
                Duration::from_secs(10),
                TestTask::new(),
            )
            .await;
    }

    #[tokio::test]
    #[should_panic(expected = "Offset must be strictly less than interval!")]
    async fn offset_exceed_interval() {
        TaskManager::default()
            .add_offset(
                "Fails",
                Duration::from_secs(10),
                Duration::from_secs(20),
                TestTask::new(),
            )
            .await;
    }

    #[tokio::test]
    async fn run_cancelled() {
        // init_logging();
        let manager = TaskManager::new(50);
        let task = TestTask::new();

        MockClock::set_system_time(Duration::from_secs(0));
        manager
            .add("Test", Duration::from_millis(100), task.clone())
            .await;

        let cancel = CancellationToken::new();

        let mut run = spawn({
            let cancel = cancel.clone();
            async move { manager.run_with_cancel(cancel).await }
        });

        let mut test = spawn({
            let cancel = cancel.clone();

            async move {
                MockClock::set_system_time(Duration::from_millis(120));
                sleep(Duration::from_millis(100)).await;
                assert_eq!(task.count().await, 1);
                MockClock::set_system_time(Duration::from_millis(220));
                sleep(Duration::from_millis(100)).await;
                assert_eq!(task.count().await, 2);

                cancel.cancel();
                sleep(Duration::from_millis(100)).await;
                panic!("Cancel did not stop manager");
            }
        });

        select! {
            res = &mut run => {
                if res.is_err() || !cancel.is_cancelled() {
                    panic!("Manager stopped unexpectedly: {res:?}");
                }
            }
            res = &mut test => {
                run.abort();
                res.unwrap();
            }
        }
    }
}
