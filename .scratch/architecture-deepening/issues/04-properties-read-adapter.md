# 04 — Properties per-PID read adapter

Status: resolved
Type: task
Blocked by: —

## Goal

One /proc reader shared by the tick collector and the properties detail
window; the seam distinguishes dead from degraded.

## Friction

Two /proc readers produce two representations of the same process:

- snapshot collector (`snapshot/src/linux/processes.rs:33-64`): `cmdline` is
  a joined `String`, `state` is a `char` derived at render
  (`components/theme.rs:32`).
- properties `collect()` (`properties/src/linux.rs:14-247`): `cmdline` is a
  `Vec<String>`, `state_label` precomputed (~202).

Same files read twice (stat, status, cmdline, environ, cgroups, io), same
math re-implemented (cpu-tick sum `linux.rs:86-87` vs
`snapshot/linux/processes.rs:142-147`; uid→username fallback
`linux.rs:22-24` vs `processes.rs:384-388`; hertz conversion). The window
starts from scratch, so the classification the collector computed (GUI flag,
electron, service-ness) is unavailable to it — the two views of one process
can disagree.

The interface to rendering is `Option<ProcessProperties>` over mpsc
(`window/mod.rs:31-44`), where `None` means both "process died" and "read
failed" — one signal, two meanings.

Untested pure helpers: `limit_to_str` (`linux.rs:6-11`), command re-quoting
(`window/linux.rs:157-168`), environ filter/sort (`window/linux.rs:377-389`).

Note: the window's own wake-channel idiom (third copy) is endorsed by
ADR-0003 — leave the idiom, don't dedupe it here.

## Solution sketch

- A shared per-PID read adapter (stat/status/cmdline/environ once) consumed
  by both the tick collector and the detail window.
- `ProcessProperties` construction moves to the data side; the UI consumes a
  ready record.
- The seam distinguishes `Err` (read failure / permission) from `Dead`.

## Wins

- one representation, one place for the delta/hertz math.
- pure helpers become testable.
- two adapters at the read seam justify it (tick + detail).

## Acceptance

- [x] Both collectors share the stat/status/cmdline read path.
- [x] Read failure renders differently from "process exited".
- [x] `limit_to_str` / re-quoting / environ filtering have unit tests.

## Answer

Implemented. Design settled by grilling (rounds 1–4):

- **`ProcBasics`** (`snapshot/src/linux/proc_basics.rs`): the shared per-PID
  read consumed by the tick collector *and* the properties window — stat +
  status + cmdline read once (cmdline skipped for kernel threads), plus the
  shared derivations: `name()` (argv0 basename → comm), `cpu_tick_sum()`,
  `cpu_time_secs()`, `username_for_uid()`. The two views of a process now
  derive identity from one place. `environ` stays on-demand (the tick
  collector's kthread skip + per-PID GUI cache gate it); single-consumer
  fields (exe, cwd, limits, smaps, fd, io) stay consumer-side — the eager
  set is only what both consumers read every time, so a properties-only
  field can never balloon the tick collector's reads.
- **`ReadError`** (`snapshot` lib.rs, shared): `Dead` (exited/never
  existed) vs `Unavailable` (exists but unreadable — permissions). Mapped
  from `procfs::ProcError` at `ProcBasics::read`.
- **Seam**: `collect(pid) -> Result<ProcessProperties, ReadError>`. The
  window stops polling + shows "(exited)" on `Dead`; on `Unavailable` it
  keeps polling and shows a stale-data warning banner ("could not be read
  (permissions?)") instead of mislabeling the process dead. macOS/Windows
  stubs return `Err(ReadError::Dead)`.
- **Properties → snapshot dep** added (data-pure; ADR-0001 intact).
- **Shared math**: the tick collector's cpu-percent delta and the window's
  cpu-tick sum both go through `ProcBasics`; `get_user_name` removed in
  favor of `username_for_uid`.
- **Pure helpers extracted + tested**: `quote_command` and
  `filter_env_vars` moved to `properties/src/window/helpers.rs` (a gpui-free
  module — they were causing a recursion-limit hit inside the element-heavy
  `window/linux.rs`), `limit_to_str` tested in the data side. 278 passing.
- Docs updated: properties CONTEXT.md.
