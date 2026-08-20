# 03 — Renice

Status: needs-triage
Type: task
Blocked by: 02 (batch variant only)

## Goal

Change a process's niceness (−20..19) from the context menu. The properties
window already reads `nice`; this adds the write verb.

## Design sketch

- Context menu → "Change Priority…" → small input dialog pre-filled with the
  current nice value; `setpriority()` via libc. One syscall, zero per-tick cost.
- Keyboard shortcut for quick open.
- Batch variant: hangs off 02 when >1 selected.
- Surface permission errors; renice of another user's process needs root —
  decide later whether the existing pkexec elevation prompts for it.

## Acceptance

- [ ] Renice a process; new value shows next tick.
- [ ] Batch renice over a selection applies to all.
