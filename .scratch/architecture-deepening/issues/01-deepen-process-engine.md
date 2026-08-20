# 01 — Deepen the process engine

Status: resolved
Type: task
Blocked by: —

## Goal

Collapse the filter/search/sort/pin/cumulative pipeline behind one entry
point so the engine's interface is small and the cache protocol stops
leaking.

## Friction

`crates/core/src/processes/delegate.rs` (1363 lines) bundles 8
responsibilities: filter predicates (`proc_matches`, ~421-484), the
filter+search+sort pipeline (`filtered_sorted_rows`, ~228-403), cumulative
aggregation (~139-225), descendant counting (~71-113), ancestor walking
(~44-136), pin handling (~388-418), the lazy pid-index cache (~32-41), and
column visibility (~23-30). Its interface is 14 pub methods + 7 pub fields
(~12-20), four of them `Rc<RefCell<Option<…>>>` caches (`cum_cache`,
`pid_index`, `descendant_counts` + `ViewState.cached`).

Callers must know the (timestamp, generation) cache key, the RefCell borrow
discipline, and the manual invalidation protocol — and call things in the
right order every frame:

- `ProcessesTab::get_delegate()` hand-assembles 7 fields per call
  (`processes/src/tab.rs:174-184`), invoked repeatedly per frame.
- `set_snapshot` (`tab.rs:129-134`) is the only invalidator of the three
  cache cells, and nothing enforces a snapshot swap goes through it. Tests
  only pass because the cache starts `None`.
- `render_td` (`table_delegate.rs:113-118`) borrows `cum_cache` directly,
  relying on `filtered_sorted_rows` having run first that frame — the
  cumulative logic has no locality (compute in the delegate, read in the
  render cell).
- Sorting is an 80-line comparator (~301-386) with a hard-coded `col >= 5`
  gate (~293); column identity is simultaneously a raw `usize`
  (`ViewState.sort_col`), a 1-based index (`SortColumn::to_col_index`), and a
  10-column UI match.
- `filtered_sorted_rows` mutates `ViewState.cached` from `&self` (~400-402).

## Solution sketch

- One `rows()` entry point owning the whole pipeline (snapshot invalidation,
  filter, search, sort, pin, cumulative map).
- Pub cache cells become private derived values; `set_snapshot` and
  `ViewState::mutate` are the only two invalidators, both inside the seam.
- Sort comparator keyed on `SortColumn`, not a raw index.

## Wins

- locality: invalidation bugs concentrate in one module.
- leverage: one interface, ~6 call sites + all tests.
- interface is the test surface — tests stop replicating the cache protocol.

## Acceptance

- [x] Interface shrinks to ~3 calls; no pub `Rc<RefCell<Option<_>>>` fields.
- [x] Swapping a snapshot without going through `set_snapshot` is impossible
      or caught by the type system.
- [x] All existing delegate tests pass unchanged; new test for
      mid-frame snapshot swap.

## Answer

Implemented. Design settled by grilling (rounds 1–2, all recommendations
accepted):

- **`ProcessEngine`** (`core/processes/engine.rs`): private snapshot (swap
  only via `set_snapshot`), `config`, `init_system`, `view_state`, and four
  private caches (rows, cumulative map, pid index, descendant counts) as
  inner `RefCell`s. All methods take `&self` — callers hold an immutable
  engine borrow while calling anything; no re-entrant borrow possible.
- **`ProcessTableDelegate`** (`core/processes/delegate.rs`) shrinks to a
  thin handle `{ engine: Rc<RefCell<ProcessEngine>> }` forwarding: `rows()`,
  `cum(pid)`, `count_descendants_of`, `ancestor_chain_of`, `pinned_pid`,
  `pid_filter_mode`, `is_col_hidden`, `snapshot()`, plus the walks
  (`is_descendant_of`, `descendant_pids_of`, `pid_to_ppid_map`,
  `proc_matches`) kept callable for `build_tree` (issue 02 consolidates).
- **Invalidation protocol**: `set_snapshot` resets all four caches (snapshot
  is private, so it's the only swap path — type-system enforced); `rows()`
  is cached on `ViewState.generation`. The `(timestamp, generation)` key is
  gone; `ViewState.cached` moved into the engine.
- **Cumulative**: computed lazily once per snapshot whenever
  `resource_view_mode == Cumulative` (fixes the bug where resource columns
  showed self values when sorted by name in Cumulative mode). Render reads it
  via `cum(pid) -> Option`, falling back to own values in SelfOnly.
- **Sort keyed on `SortColumn`**: `ViewState.sort_col` is now `SortColumn`;
  the `col >= 5` magic gate and the raw-index match are gone. UI
  `perform_sort` converts via `SortColumn::from_col_index`.
- **Wiring**: `ProcessesTab` drops `snapshot_cell`/`cum_cache`/`pid_index`/
  `descendant_counts`, holds `engine`; `get_delegate()` clones the Rc;
  `set_snapshot` forwards. `table_delegate`/`status_bar`/`breadcrumbs`/
  `tree`/`tree_view`/`toolbar` read via `rows()`/`cum()`/`snapshot()`.
- Tests: behavior-preserved (all assertions identical; harness builds the
  engine, vram snapshot swaps go through `set_snapshot`, `rows()`/`cum()`
  renames). New `rows_reflect_snapshot_swapped_mid_frame`. 273 passing.
