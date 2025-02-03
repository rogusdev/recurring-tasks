# recurring-tasks -- Recurring Tasks Manager
Rust crate to build an app that (simply) runs recurring, periodic tasks -- effectively cronjobs, in a dedicated process. And will not run a task that is already (still) running.

This is NOT a good choice for moderate high frequency (sub second)! Tasks can drift off schedule by as much as a few seconds. This is a best effort to be in the ballpark, not for super precision. Perhaps this will be improved in the future.
