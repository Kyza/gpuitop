# Window Picker

"Click a window to find its process": a crosshair tool that identifies the focused window (X11 or wayland) and maps it to a filter keyword that pre-fills the process search.

## Language

**PickedWindow**:
The picker's result: `AppId(String)` (wayland/x11 class) or `Pid(i32)`. Consumed as a search keyword — the last dotted segment of the app_id, or the pid as a string. Picking narrows the table; it doesn't select a row.
_Avoid_: "window selection" (there is no row selection)

**GPUITOP_APP_ID**:
`com.github.kyza.gpuitop` — the app's own reverse-DNS id, used as the window `app_id` and as the self-exclusion filter so the picker never targets its own window.

**pick_window**:
Dispatches on `WAYLAND_DISPLAY` vs `DISPLAY` to the wayland or X11 path, then polls the focused window every 200ms up to 10s on a spawned thread.
_Avoid_: "crosshair" (the cursor affordance, not the protocol logic)

**Wayland pick**:
Uses the wlroots `zwlr_foreign_toplevel` protocol; identifies by `app_id` only — wayland yields no pid. Wlroots-only; won't work on compositors without the protocol.
**X11 pick**:
Reads `_NET_ACTIVE_WINDOW` then resolves `_NET_WM_PID`, falling back to `WM_CLASS`; skips its own pid and the `"gpuitop"` wm_class.

## Relationships

- **Window Picker → Processes**: `PickedWindow` pre-fills the process search input.
- **X11 vs Wayland**: two protocols, two identification strategies (pid vs app_id), same poll-loop shape.
