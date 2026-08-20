# 02 — Batch actions

Status: needs-triage
Type: task
Blocked by: 01

## Goal

Act on the selected set at once. When more than one row is selected, offer bulk
variants — End Process, Force Kill, Pause/Resume, later Renice — instead of
(or supplementing) the single-process options.

## Design sketch

- Show bulk variants when `selected_rows.len() > 1`, via a floating toolbar or
  the context menu.
- Apply one signal per pid in the set; report per-pid errors (e.g. permission).
- Batch renice lands with 03.

## Notes

- Selection must survive ticks (see 01); the bulk menu reflects the live set.
- No multi-process signal API on Linux — just a loop over pids.

## Acceptance

- [ ] Bulk kill/terminate sends the signal to every selected pid.
- [ ] Bulk variants only appear when >1 row is selected.
- [ ] Per-pid permission errors surface without aborting the rest.
