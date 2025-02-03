# recurring-tasks -- Recurring Tasks Manager
Rust crate to build an app that (simply) runs recurring, periodic tasks -- effectively cronjobs, in a dedicated process. And will not run a task that is already (still) running.

Can support sub-second periods, but runs the risk of falling behind, depending on the system / environment this runs on/in.

Full, but easily digestible, examples are in the [examples dir](https://github.com/rogusdev/recurring-tasks/tree/main/examples/).

This is designed to be a very focused solution for building an app that has only one job: running various tasks repeatedly, forever. Look at [tokio-cron-scheduler crate](https://github.com/mvniekerk/tokio-cron-scheduler) for a much more elaborate approach, using crontab syntax.

Very important: in WSL2, Rust's `Instant` does not properly track seconds for some reason, the virtual clock seems to be just a tiny bit slow relatively to the actual clock, so it will often accumulate 2-3 extra seconds on 20 sec periods... The only way around it if you encounter this is to use the feature `system`. The default flag of `instant` is more appropriate for production use, because `system` runs the risk of panicing or just behaving strangely if the system clock changes, especially backwards (daylight savings changes, etc).
