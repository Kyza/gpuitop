# 04 — Kill-tree

Status: needs-triage
Type: task
Blocked by: —

## Goal

Signal a process and all its descendants — a distinct verb from multi-select
batch (which targets siblings). Signals don't propagate: killing a parent does
NOT signal its children, they are reparented to pid 1 and keep running.
Tree-kill makes "end this whole app" reliable.

## Design sketch

- Context menu: "End Process Tree" (SIGTERM subtree) / "Force Kill Tree"
  (SIGKILL subtree), plus signal-tree variants.
- Reuse the existing subtree walk used by pin/cumulative (`is_descendant_of`);
  collect the tree at click time — the tree can change between click and
  signal.
- Order: leaves → root, so a dying parent can't respawn children mid-signal.

## Acceptance

- [ ] Killing a parent's tree also signals every descendant.
- [ ] Works from tree and list views.
