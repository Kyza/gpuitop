# 01 — CPU affinity

Status: needs-triage
Type: task
Blocked by: —

## Goal

Pin a process to specific cores via `sched_setaffinity`.

## Notes

- Niche; deferred. The syscall is cheap — the core-picker UI is the real work.
