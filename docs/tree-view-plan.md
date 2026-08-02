# Tree View Plan

Toggle between **List View** (existing `DataTable`) and **Tree View** (gpui-component `tree()`) for the process list.

---

## New `ProcessesTab` fields (mod.rs)

| Field | Type | Purpose |
|---|---|---|
| `show_tree_view` | `bool` | List ↔ Tree toggle |
| `tree_state` | `Entity<TreeState>` | gpui-component tree widget state |
| `tree_double_click` | `Option<(Instant, String)>` | Double-click detection (time + pid-id) |

---

## `TreeData` struct (delegate.rs)

```rust
pub struct TreeData {
    pub items: Vec<TreeItem>,                      // tree children
    pub process_lookup: HashMap<i32, ProcessInfo>, // pid → info for render
    pub matched_pids: HashSet<i32>,                // direct matches (full opacity)
    pub root_proc: Option<ProcessInfo>,            // pinned PID header
}
```

---

## `build_tree()` algorithm (delegate.rs)

1. **PID scoping** — If `Filter::Pid(*)` active, collect pinned PID + all descendants into `scoped_pids`
2. **Match checking** — For each process: check non-PID filters (AND/OR) + search
3. **Display set** — `matched_pids` (direct matches) ∪ ancestors (walk ppid chain up)
4. **Children map** — Group by ppid, sort each group by PID ascending
5. **Root** — If pinned: `root_proc = pinned_pid` (header), children = its visible descendants. Otherwise: top-level = procs whose ppid ∉ display set
6. **Auto-expand** — Ancestors of matched PIDs are expanded; matched PID itself is NOT expanded
7. **TreeItems** — ID: `"pid-{pid}:{match|ancestor}"`, `.is_folder` if has children, `.expanded` per auto-expand set

---

## `render_tree_view()` (render.rs)

- Calls `delegate.build_tree()` → updates `self.tree_state`
- If `root_proc` exists: renders non-collapsible header with tag icons + `(PID) name`
- Renders `tree()` component with:
  - Each node: tab-indented (`16px × depth + 12px`), tag icons, `(PID) name`
  - Grayed opacity (0.4) for ancestor-only nodes
  - Double-click (400ms same-id): sets `Filter::Pid(pid)`, clears search per config, notifies
  - Right-click: existing `build_process_menu()`

---

## Toolbar changes (render.rs)

- Add "List"/"Tree" toggle button alongside AND/OR in controls group
- Hide Self/Cumulative button when `show_tree_view`
- Hide Direct/All toggle in breadcrumb when `show_tree_view`

---

## Files changed

| File | Lines changed | What |
|---|---|---|
| `mod.rs` | +15 | 3 fields, `toggle_view_mode()` |
| `render.rs` | +140/−10 | Toggle button, `render_tree_view()`, conditional hide, imports |
| `delegate.rs` | +130 | `TreeData`, `build_tree()`, `descendant_pids_of()`, `pid_to_ppid_map()` |

---

## Unchanged

`table.rs`, `state.rs`, `theme.rs`, `fuzzy.rs`, `chips.rs`, `tests.rs`, status bar, context menu, filter logic
