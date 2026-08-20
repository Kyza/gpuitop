# 02 — Consolidate the process-graph walks

Status: resolved
Type: task
Blocked by: 01

## Goal

One process-graph module (descendants, subtree counts, ancestor chains,
match-set) so "is X in Y's subtree" is answered once per snapshot.

## Friction

"Descendant of" is implemented three ways:

1. pid-index walk — `core/processes/delegate.rs:44` (`is_descendant_of`).
2. linear scan — `core/model.rs:431` (`is_descendant_of_flat`), consumed by
   `Filter::label` (~415-427) for the pin's label count.
3. a third copy in the test harness — `core/processes/tests.rs:36`.

Subtree counts live in two algorithms with two staleness behaviors:

- cached `count_descendants_of` (`core/processes/delegate.rs:105`) — the
  `(+N)` badge in the list view (`table_delegate.rs:166`) and the breadcrumb
  badge (`breadcrumbs.rs:88`).
- memoized `count_descendants` (`processes/src/tree.rs:307-339`) — the tree
  view badge (`tree_view.rs:116-117`).

The tree builder re-runs the search+filter predicate (`tree.rs:93-137`, the
same nucleo dance as `filtered_sorted_rows`) because the flat row list
doesn't carry "is match vs is ancestor" — so the filter pipeline has a second
copy in the UI crate. Untested: `count_descendants`, `scoped_pids`
(`tree.rs:72-81`), root-pinning (`tree.rs:181-185`).

## Solution sketch

- A graph module built once per snapshot exposing: descendants of, subtree
  counts, ancestor chain, ancestors-to-keep/expand, and the match-set.
- Tree builder consumes the pipeline's match-set instead of re-running
  filtering.
- The pin's label count and its breadcrumb badge read the same number.

## Wins

- deletion test: 3 implementations → 1; the cycle guard fixes everywhere.
- locality: the pin label and its badge stop disagreeing.
- tree.rs's untested predicate copy is deleted.

## Acceptance

- [x] One `is_descendant_of`-equivalent in the codebase.
- [x] Breadcrumb `(+N)` and label count agree for the same pin.
- [x] Tree builder has no filter predicate of its own.

## Answer

Implemented. Design settled by grilling (rounds 1–3):

- **`ProcessGraph`** (`core/processes/graph.rs`): built lazily once per
  snapshot, owned by the engine as a private cache reset by `set_snapshot`.
  Absorbs the old `pid_index` + `descendant_counts` caches. Holds pid→index,
  `children_by_ppid`, full `subtree_counts`; exposes `is_descendant_of`,
  `descendants_of`, `subtree_count_of`, `ancestor_chain_of`,
  `with_ancestors`, `ancestors_to_expand`, `children_of`,
  `display_subtree_counts`. All walks cycle-guarded (replaces the cap-100).
  `core/processes/tree.rs` folded in as methods (tests moved).
- **Pid filter = scope restriction, both views**: `proc_matches`'s Pid arm
  deleted. Pipeline computes the `match set` = search + non-Pid filters
  (mode-combined); `rows()` = match set ∩ mode-aware scope + sort +
  pin-to-top; `tree_match_set()` = match set ∩ full descendant scope. One
  generation-keyed cache `{gen, rows, matched}` — rows and match set are one
  pass. Fixes the list/tree divergence under `Or` + pin (pin always
  restricts).
- **Tree** (`processes/src/tree.rs`): consumes `tree_match_set()` + the
  graph; its own search/filter predicate, `scoped_pids`, children map, and
  `count_descendants` are deleted. Roots at one node per pin. TreeData keeps
  `shown_descendant_counts` (graph) and gains `subtree_counts` (full).
- **Badges**: list keeps full-subtree `(+N)`; tree shows `(+shown/total)`
  always, with a tooltip ("N of M descendants shown").
- **Pin label**: `is_descendant_of_flat` deleted; `Filter::label`'s Pid arm
  returns the name; toolbar formats "name (+N children)" from
  `count_descendants_of` — the breadcrumb badge already read the same cached
  number, so they agree by identity.
- **Status bar**: tree mode counts `tree_match_set()` (matches the visible
  tree); list mode counts rows.
- Tests: deleted the tests.rs harness copies (`is_descendant_of`,
  `proc_matches_standalone`) and the model.rs `is_descendant_of_flat`
  suite; reworked delegate Pid `proc_matches` tests into scope tests
  (`rows_pid_scope_*`, `match_set_*`, `tree_match_set_*`); added focused
  `ProcessGraph` tests incl. cycle guards. 266 passing.
- Behavior changes: under `Or` + pin the list no longer shows out-of-subtree
  matches; multiple pins render one tree root per pin.
