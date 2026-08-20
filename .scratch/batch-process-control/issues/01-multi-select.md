# 01 — Multi-select

Status: needs-triage
Type: task
Blocked by: —

## Goal

Select a set of process rows (primarily siblings) to drive batch actions.

## Starting point

`docs/multiselect-plan.md` is **very outdated** — it predates the crate
restructure and the core/UI delegate split. Use it as a hint only; re-derive
the design from the current architecture before implementing:

- Old paths (`src/tabs/processes.rs`) are now `crates/processes/`.
- Selection is UI state → belongs on the **UI** delegate newtype
  (`processes/delegate.rs`), not the data-pure core delegate
  (`core/processes/delegate.rs`).
- Verify what the current gpui_component `TableDelegate` / `TableState` expose
  for row clicks, modifiers, and key nav — the plan's interception approach may
  not match today's API.

## Design sketch (from the old plan)

- `selected_rows: Rc<RefCell<BTreeSet<usize>>>` shared between delegate and
  tab so render and actions both read it.
- Plain click: clear set, select clicked row only. Ctrl+click: toggle row.
  Shift+click: range-select anchor → clicked row (via
  `ClickEvent.modifiers.shift/command`).
- Render: selected rows get `bg(primary.opacity(0.08))` + left border.

## Deferred

- Keyboard range-select (shift+arrow): the table owns key nav internally; would
  need `TableEvent::SelectedRow` tracking or `TableState` binding interception.
  Mouse-only to start.

## Acceptance

- [ ] Select N rows with ctrl/shift+click; selection survives a re-render tick.
- [ ] Plain click collapses selection to one row.
- [ ] Selection is cleared/consistent when the filtered row set changes.
