# Architecture deepening — milestone spec

Surface architectural friction found in the 2026-08-19 architecture review
(`/tmp/architecture-review-1787148687.html`) and deepen the shallow modules
behind it. Vocabulary: module, interface, implementation, depth, seam,
adapter, leverage, locality.

## Ordering

1. **Deepen the process engine** — collapse the filter/search/sort/pin/
   cumulative pipeline behind one entry point; the pub cache cells become
   private derived values. Highest leverage: hottest path, most tested.
2. **Consolidate the process-graph walks** — three descendant walks, two
   subtree counts, and the tree's duplicated filter predicate collapse into
   one graph module. Unblocked by 01's match-set.
3. **Honor the config at the collector seam** — thread `VramPolling` into
   `CollectorState`; prune the field-list seam; delete dead `core_history`
   and `pid_alive`.
4. **Properties per-PID read adapter** — one /proc reader for the tick
   collector and the detail window; the seam distinguishes dead from
   degraded.
5. **One config home** — single shared `Config` handle; the mutations
   gateway is the sole writer; kill the hand-sync mirror handlers.

## Principles

- **Data-pure core stays** (ADR-0001): deepening keeps pure logic in core;
  gpui stays out of data crates.
- **Wholesale invalidation stays** (ADR-0004): the deepening encapsulates the
  protocol behind the seam, it does not re-litigate it.
- **The interface is the test surface**: tests cross the same seam as
  callers.
- **Deletion test on every candidate**: complexity must concentrate, not
  move.

## Status

- 01: `resolved` (implemented).
- 02: `resolved` (implemented).
- 03: `resolved` (implemented).
- 04: `resolved` (implemented).
- 05: `resolved` (implemented).
