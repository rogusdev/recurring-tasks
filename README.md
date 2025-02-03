# recurring-tasks -- Recurring Tasks Manager
Rust crate to build an app that (simply) runs recurring, periodic tasks -- effectively cronjobs, in a dedicated process. And will not run a task that is already (still) running.

Can support sub-second periods, but runs the risk of falling behind, depending on the system / environment this runs on/in.

Full, but easily digestible, examples are in the [examples dir](https://github.com/rogusdev/recurring-tasks/tree/main/examples/).

This is designed to be a very focused solution for building an app that has only one job: running various tasks repeatedly, forever. Look at [tokio-cron-scheduler crate](https://github.com/mvniekerk/tokio-cron-scheduler) for a much more elaborate approach, using crontab syntax.

This will panic or behave strangely if the system time changes while it is running!
