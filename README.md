# recurring-tasks -- Recurring Tasks Manager
Rust crate to build an app that (simply) runs recurring, periodic tasks -- effectively cronjobs, in a dedicated process. And will not run a task that is already (still) running. Supports sub-second periods.

Full, but easily digestible, examples are in the [apps dir](https://github.com/rogusdev/recurring-tasks/tree/main/apps/).

This is designed to be a very focused solution for building an app that has only one job: running various tasks repeatedly, forever. Look at (not mine, just shoutout) [tokio-cron-scheduler crate](https://github.com/mvniekerk/tokio-cron-scheduler) for a much more elaborate approach, using crontab syntax.

Very important: in WSL2, Rust's `Instant` does not properly track seconds for some reason, the virtual clock seems to be just a tiny bit slow relatively to the actual clock, so it will often accumulate 2-3 extra seconds on 20 sec periods... This crate uses Tokio, which is built on Instant, so this drift will be visible in WSL.

Docs.rs [documentation](https://docs.rs/recurring_tasks/latest/recurring_tasks/).
