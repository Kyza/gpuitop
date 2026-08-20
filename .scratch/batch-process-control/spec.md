# Batch Process Control — milestone spec

Turn gpuitop from a watcher into a controller: multi-select the process table,
then act on the selection — batch kill, batch renice — plus the control verbs
renice and kill-tree as first-class single-row actions.

## Ordering

1. **Multi-select** (foundation) — nothing else in this feature lands without it.
2. **Batch actions** — kill/terminate/renice over the selection.
3. **Renice** — single-row verb; batch variant hangs off 02.
4. **Kill-tree** — signal a process and all descendants; a distinct verb from
   batch (multi-select targets siblings; kill-tree targets a parent's subtree).

## Principles

- **Table stays cheap**: per-tick columns keep their current /proc reads; verbs
  are one-syscall actions with zero per-tick cost.
- **Usability bar**: every verb reachable in ≤2 clicks from a row; keyboard
  shortcuts for the common ones; context menu grouped by verb (Control / Signals).
- **Selection ≠ tree**: multi-select is sibling-oriented; kill-tree is
  parent→child. Both exist.

## Deferred / rejected

- CPU affinity, scheduling policy: issue-tracked in `process-control-depth/`.
- Run-new-task: rejected (a terminal beats any launcher we'd build).
- Real macOS/Windows: out of scope (ADR-0007); Windows planned later.

## Status

- All tickets: `needs-triage`.
