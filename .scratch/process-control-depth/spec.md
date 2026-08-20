# Advanced process control — deferred

Deferred single-process control verbs that don't make the current milestone.
Fully specified when they climb the priority list.

- CPU affinity (`sched_setaffinity`)
- Scheduling policy (SCHED_FIFO/RR/BATCH, needs privileges)
