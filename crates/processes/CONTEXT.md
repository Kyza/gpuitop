# Processes

The process list view — the main tab. Renders the filtered/sorted process set as either a tree or a flat list, plus toolbar, breadcrumbs, chips, and the per-process context menu. All the heavy lifting lives in the core `ProcessEngine`; this crate is the GPUI skin over it.

## Language

**ProcessesTab**:
The GPUI view hosting the whole panel: the shared `Rc<RefCell<ProcessEngine>>`, the `ViewState` it wraps, table/tree state, and window-picking plumbing. Renders toolbar → tree-or-list → status bar.

**engine**:
The shared `ProcessEngine` (`core/processes/engine.rs`) holding the current snapshot and all derived caches; swapped each tick via `set_snapshot` (the tab forwards to `self.engine.set_snapshot`). Reads that need the snapshot go through `self.engine.snapshot()`. Held as `Rc<ProcessEngine>` — no outer `RefCell`: the engine keeps all its state behind inner `RefCell`s and exposes `&self` methods only.
_Avoid_: "snapshot cell" (the old `Rc<RefCell<Rc<SystemSnapshot>>>` field is gone; the engine owns the snapshot privately)

**ProcessTableDelegate (UI)**:
The newtype `ProcessTableDelegate(pub Rc<ProcessEngine>)` that lets the engine implement gpui_component's `TableDelegate` (orphan rule). A pure deref wrapper — no state of its own.
_Avoid_: confusing it with the engine (all state lives in the engine)

**Config store**:
The shared `ConfigStore` held by the tab and the engine. Column visibility is read live from it (`is_col_hidden` → `store.get().processes`), so column edits in Settings apply to the running table without restart.
_Avoid_: "column_visibility" — the old per-tab `ProcessesConfig` clone is gone

**rows**:
The list pipeline result on the engine (`rows()`): the pipeline's `match set` intersected with the mode-aware Pid scope, then sorted, then pin-to-top — cached on `ViewState.generation`. In tree mode the status bar instead counts `tree_match_set()` so "filtered of total" matches what the tree shows.
_Avoid_: "filtered_sorted_rows" (the old name; the pipeline collapsed behind `rows()`)

**tree match set**:
The engine's `tree_match_set()`: the pipeline match set restricted to the pins' full descendant scopes (the tree always shows all descendants, ignoring `pid_filter_mode`). The tree builder consumes this — it has no filter predicate of its own.

**Pin / breadcrumbs**:
The pinned PID (a `Filter::Pid`) is forced to the top of the list and scopes both views to its subtree; breadcrumbs render its ancestor chain root→target, each clickable to re-pin, with a Direct/All mode toggle (list view only). The pin's child count in the toolbar chip and the breadcrumb badge read the same engine-cached `count_descendants_of`, so they can never disagree.
_Avoid_: "follow" — pin is the established term

**Chips**:
The state-menu items (`chips.rs`) that toggle `Filter::ProcessState` checkboxes.

**Context menu**:
Right-click per-process menu: End/Force Kill/Pause/Resume, a signal list (HUP…USR2), Copy PID, and "Properties" (opens a `PropertiesWindow`).

**Signal**:
A Unix signal sent to a process via `libc::kill` from the context menu.

**Multi-select**:
A set of selected process rows (UI state on the processes delegate) driving
batch actions. Sibling-oriented: plain click clears and selects one, Ctrl+click
toggles, Shift+click ranges.
_Avoid_: "selection" alone when you mean the pinned PID (pin is a `Filter::Pid`)

**Batch action**:
One verb applied to every process in the multi-select set (kill, terminate,
pause/resume, renice). Distinct from **kill-tree**: batch targets siblings.
_Avoid_: "bulk" (fine in prose; Batch is the codebase term)

**Renice**:
Change a process's niceness (−20..19) via `setpriority`; a context-menu verb.
Distinct from the read-only `nice` shown in Properties.

**Kill-tree**:
Signal a process and all its descendants (leaves first, so a dying parent
can't respawn children mid-signal). A distinct verb from batch actions: batch
targets siblings, kill-tree targets a parent's subtree. Needed because signals
don't propagate — children are reparented to pid 1 and keep running.
_Avoid_: conflating with batch kill

**Tree**:
The process hierarchy derived from `ppid` (never stored — always derived from the flat snapshot per rebuild via the graph). Consumes `tree_match_set()`, expands to include ancestors so matches are never orphans, and roots at one node per pinned PID when pins are set. The `(+shown/total)` badge next to each node shows how many of its descendants are shown vs its full subtree. Non-matching ancestor rows are dimmed and forced expanded.
_Avoid_: "tree data" (that's the implementation type `TreeData`)

**preserve_expand_from**:
Re-applies expand/collapse state from the previous `TreeData` onto freshly-built items by id — because `set_items()` wipes expand state.
_Avoid_: "restore expansion"

**last_seen_seeds**:
The tab's `ProcessSeeds` snapshot (default_sort + default_view_mode). Diffed at the top of `render` via `apply_seed_changes`; on an actual change the sort re-seeds `ViewState` and the default view flips `show_tree_view`. `resource_view_mode` / `pid_filter_mode` stay toolbar-owned seeds, never re-applied live.
_Avoid_: "seed cache" (it is a diff baseline, not a cache)

**List**:
The flat alternative to the tree, using the same hierarchy as descendant badges (`(+N)`) instead of nesting.

**Toolbar**:
Filter bar: view-mode toggle, search input (with window picker), type-filter toggles, username toggles, AND/OR, resource view (Self/Cumulative), state dropdown, PID breadcrumbs.

**Status bar**:
Bottom bar: "filtered of total" plus the active filter count and mode.

## Relationships

- **engine → delegate**: `get_delegate()` clones the shared engine Rc into a thin wrapper; all state lives in the engine.
- **graph → tree**: `build_tree` consumes `tree_match_set()` plus the graph's walks (`with_ancestors`, `ancestors_to_expand`, `children_of`, `display_subtree_counts`) — no filter predicate of its own.
- **Window picker → toolbar**: `PickedWindow` pre-fills the search input as a filter keyword.
- **Processes → Properties**: the context menu opens a `PropertiesWindow` for the selected PID.
- **Invalidation**: `set_snapshot` resets the engine's caches (including the graph); `ViewState::mutate` bumps `generation`, invalidating cached rows and the cached tree.
