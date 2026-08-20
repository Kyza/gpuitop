# PID-reuse-safe caches via `starttime`

Expensive per-process results that never change at runtime (the GUI flag) are cached per PID, but only reused when the process's `starttime` matches. Linux recycles PIDs; without the guard a recycled PID would inherit the previous process's cached GUI flag.

The cost: `starttime` is read alongside the cached value each tick. The benefit: environ reads for GUI detection happen once per process lifetime instead of every tick.
