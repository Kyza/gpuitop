# Properties

The per-process detail window: collects a full static snapshot of one process and renders it in a standalone popup, re-polling every 1500ms until the process exits.

## Language

**ProcessProperties**:
The complete static snapshot of one process: pid/ppid, name, command, state, user/groups, priority, CPU ticks, all `vm_*` and `io_*` fields, limits, exe/cwd/root, cgroups, environ, fds, context switches, and an smaps summary. Read via the shared `ProcBasics` adapter (stat/status/cmdline) plus the per-PID reads the window alone needs.
_Avoid_: "process info" (the window is the UI; this is the data)

**PropertiesWindow**:
The standalone gpui window hosting the detail view for one PID. Polls `collect()` on a background thread and uses a wake channel to trigger re-render on new data; stops polling and shows an "(exited)" banner when the process is `Dead`, and keeps polling with a stale-data warning when the read is `Unavailable` (permissions).
_Avoid_: "detail window" (that's the user-facing name; Properties is the codebase term)

**LimitEntry**:
One resource limit with soft/hard string values ("unlimited" or a number).
**SmapsSummary**:
Summarized per-process memory from `/proc/<pid>/smaps_rollup`: PSS, USS, swap, shared/private clean+dirty, referenced, anonymous. USS is derived as `private_clean + private_dirty`.
_Avoid_: "smaps" alone (the summary is the rollup)

**Properties tabs**:
Overview, Memory, I/O, Environment, FDs, Limits. Environment has name + content filter bars; Overview reconstructs the command with shell-style quoting.

**Standalone mode**:
The `--properties <PID>` CLI mode opens only the properties window (client-decorated, titled "Properties — PID N") with no main window; the app quits when the window closes.
_Avoid_: "popup mode" (popups live over the main window; this replaces it)

## Relationships

- **Properties → Snapshot**: identity fields (name/user/ticks/hertz) come from the shared `ProcBasics` adapter; the dead-vs-degraded `ReadError` seam is snapshot's type.
- **Properties → Core**: `ProcessProperties` is a distinct, richer type alongside `ProcessSnapshot` (the per-row list data).
- **Processes → Properties**: the context menu opens a `PropertiesWindow` for the selected PID.
- **Linux-only**: macOS/Windows `collect()` returns `Err(ReadError::Dead)` (see ADR-0007).
- **Wake-channel pattern**: the same event-driven render driving as the main app (see ADR-0003).
