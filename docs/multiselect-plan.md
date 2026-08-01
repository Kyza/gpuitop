# Multi-Select Plan

## State
Add `selected_rows: Rc<RefCell<BTreeSet<usize>>>` to `ProcessTableDelegate`, shared with `ProcessesTab` so the delegate and render can both access it.

## Selection Mechanics

| Action | Behavior |
|--------|----------|
| Plain click | Clear set, select clicked row only |
| Ctrl+click | Toggle row in set |
| Shift+click | Range-select from anchor to clicked row |

Intercepted via `on_click` handlers on each row in `render_tr`. GPUI's `ClickEvent` exposes `modifiers.shift` and `modifiers.command` (ctrl on Linux).

## Keyboard Range-Select (risky)
The table owns up/down key nav internally. To support shift+arrow range extension, we'd need to either:
- Wrap the table in a `on_key_down` handler, tracking `current_row` from `TableEvent::SelectedRow`
- Or intercept `TableState` key bindings

Recommend: start with mouse-only multi-select. Keyboard range-select can follow later.

## Rendering
In `render_tr`: if row is in the set, apply `bg(cx.theme().primary.opacity(0.08))` with a left border.

## Bulk Actions
When `selected_rows.len() > 1`, show "Kill Selected" in a floating toolbar or in the context menu instead of/supplementing single-process options.

## File Changes
- `src/tabs/processes.rs`: add field to delegate struct, update `render_tr`, add click handlers, add bulk menu logic
- `src/widgets/process_context.rs`: add bulk variants of kill/terminate actions
