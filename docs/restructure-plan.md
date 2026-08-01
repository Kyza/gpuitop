# Codebase Restructure Plan

Break up monolithic files (~1,900-line `processes.rs`, ~920-line `collect.rs`) into
small, focused modules. No logic changes — pure file-level reorganization, import
path updates, and extraction of render helpers to separate files.

Goals:
1. No single file over ~500 lines (exceptions: model.rs at 361 lines is fine)
2. Platform-specific code in `platform/` for future Windows/macOS builds
3. UI code separated into page/tab/component style
4. Helpers in their own files
5. All tests keep passing; only necessary import path updates

---

## New Directory Structure

```
src/
├── main.rs                           # unchanged (61 lines)
├── app.rs                            # imports updated only
├── model.rs                          # unchanged (361 lines)
├── config.rs                         # unchanged (158 lines)
│
├── platform/                         # NEW: platform abstraction layer
│   ├── mod.rs                        # cfg-based re-export of current platform
│   └── linux/
│       ├── mod.rs                    # re-exports all sub-modules
│       ├── collector.rs              # SystemCollector struct + CPU/mem/disk/net (~400 lines)
│       ├── process.rs               # collect_processes + electron helpers (~400 lines)
│       ├── gpu.rs                    # VRAM maps + detect_gpu (~130 lines)
│       └── wayland.rs               # Wayland toplevel queries (moved from src/wayland.rs)
│
├── tabs/
│   ├── mod.rs                        # Tab enum + re-exports (unchanged, 14 lines)
│   ├── performance.rs               # PerformanceTab (unchanged, 270 lines)
│   │
│   ├── settings/                     # NEW: split settings into pages
│   │   ├── mod.rs                    # SettingsTab struct + save + setting_pages + Render (~80 lines)
│   │   ├── general.rs               # general_page (~105 lines)
│   │   ├── processes.rs             # processes_page (~150 lines)
│   │   ├── columns.rs               # columns_page (~75 lines)
│   │   └── about.rs                 # about_page (~40 lines)
│   │
│   └── processes/                    # NEW: split 1,867-line monolith
│       ├── mod.rs                    # ProcessesTab struct + basic methods (~120 lines)
│       ├── state.rs                  # CumulativeResources + ViewState (~55 lines)
│       ├── delegate.rs              # ProcessTableDelegate (tree, filter, sort) (~420 lines)
│       ├── fuzzy.rs                  # fuzzy_match, nucleo_fuzzy_score, best_fuzzy_score (~45 lines)
│       ├── theme.rs                  # TagType + theme/style/color helpers (~125 lines)
│       ├── chips.rs                  # render_filter_chip + get_state_item (~165 lines)
│       ├── table.rs                  # TableDelegate impl (render_th/render_td/etc.) (~325 lines)
│       ├── render.rs                 # Render impl + toolbar/breadcrumb/status helpers (~450 lines)
│       └── tests.rs                  # All tests (~280 lines)
│
└── widgets/
    ├── mod.rs                        # unchanged (1 line)
    └── process_context.rs            # unchanged (109 lines)
```

---

## File-by-File Change Details

### 1. `src/main.rs`
- Replace `mod wayland;` with `mod platform;`
- No other changes.

### 2. `src/platform/mod.rs` (NEW)
```rust
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;
```
On non-Linux, compile-error with a `compile_error!` until that platform is implemented.

### 3. `src/platform/linux/mod.rs` (NEW)
```rust
pub mod collector;
pub mod gpu;
pub mod process;
pub mod wayland;

pub use collector::SystemCollector;
pub use gpu::detect_gpu;
pub use wayland::{get_focused_window_app_id, get_toplevels};
```

### 4. `src/platform/linux/collector.rs` (from `src/collect.rs`)
**Move:**
- `SystemCollector` struct definition
- `impl SystemCollector { new, tick, read, parse_kv, collect_cpu, collect_memory, collect_disks, collect_networks }`
- Tests: `test_collector_has_processes`, `test_collector_has_cpu_data`, `test_collector_has_memory_data`, `bench_collector_tick`

**Imports added:** `crate::model::*`, `std::collections::HashMap`, `std::fs`, `std::time::Instant`

**Lines taken:** ~400 of the 921 from `src/collect.rs`

### 5. `src/platform/linux/process.rs` (from `src/collect.rs`)
**Move:**
- `impl SystemCollector { collect_processes, parse_stat }`
- Free functions: `ticks_per_second`, `find_electron_root`, `extract_electron_app_name`, `capitalize`, `has_display_var`, `get_user_name`
- Tests: `test_processes_flat_list`, `test_gui_detection`, `test_ppid1_processes`

**Imports added:** `crate::model::*`, `super::collector::SystemCollector`, `super::gpu::build_vram_map`, `std::collections::{HashMap, HashSet}`, `std::fs`

**Lines taken:** ~400 of the 921 from `src/collect.rs`

### 6. `src/platform/linux/gpu.rs` (from `src/collect.rs`)
**Move:**
- `build_vram_map`, `build_nvidia_vram_map`, `build_rocm_vram_map`, `detect_gpu`

**Imports:** `crate::model::GpuBackend`, `std::collections::HashMap`

**Lines:** ~130 (from lines 640-722 of `collect.rs`)

### 7. `src/platform/linux/wayland.rs` (from `src/wayland.rs`)
**Move:** Entire file as-is.

**Import change:** `crate::GPUITOP_APP_ID` — stays the same.

**Lines:** 196 (unchanged)

### 8. `src/tabs/processes/mod.rs` (from `src/tabs/processes.rs`)

**Contains:**
- Module declarations: `pub mod state; pub mod delegate; pub mod fuzzy; pub mod theme; pub mod chips; pub mod table; pub mod render;` + `#[cfg(test)] mod tests;`
- `pub struct ProcessesTab` (lines 51-67)
- `impl ProcessesTab { new, set_snapshot, toggle_filter, toggle_filter_mode, toggle_pid_filter_mode, focus_input, toggle_resource_view_mode, get_delegate }` (lines 69-163, minus the Render impl)

**Imports:**
```rust
use crate::config::Config;
use crate::model::*;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{input::InputState, table::TableState, ActiveTheme, Icon, IconName};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, atomic::AtomicBool, Mutex};
```

**Lines:** ~120

### 9. `src/tabs/processes/state.rs` (NEW)

**Contains:**
- `CumulativeResources` struct (lines 1-8)
- `ViewState` struct (lines 31-41)
- `impl ViewState { mutate }` (lines 43-49)

**Imports:** `crate::model::*`, `std::rc::Rc`, `std::cell::RefCell`

**Lines:** ~55

### 10. `src/tabs/processes/delegate.rs` (NEW)

**Contains:**
- `use super::state::ViewState;`
- `use super::fuzzy::{fuzzy_match, best_fuzzy_score};` (or `use super::fuzzy::*;`)
- `ProcessTableDelegate` struct (lines 165-170)
- `impl ProcessTableDelegate { is_descendant_of, count_descendants_of, ancestor_chain_of, compute_aggregate_cumulative_map, filtered_sorted_rows, pid_filter_mode, pinned_pid, proc_matches }` (lines 172-551)

**Imports:**
```rust
use crate::model::*;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use gpui_component::table::ColumnSort;
use super::state::ViewState;
use super::fuzzy::{fuzzy_match, best_fuzzy_score};
```

**Lines:** ~420

### 11. `src/tabs/processes/fuzzy.rs` (NEW)

**Contains:**
- `fuzzy_match` (lines 553-555)
- `nucleo_fuzzy_score` (lines 557-570)
- `best_fuzzy_score` (lines 572-593)

**Imports:** `crate::model::ProcessInfo`

**Lines:** ~45

### 12. `src/tabs/processes/theme.rs` (NEW)

**Contains:**
- `TagType` enum (lines 645-655)
- `theme_dark_or_light` (lines 595-597)
- `state_info` (lines 599-622)
- `tag_color` (lines 624-643)
- `tag_icons` (lines 657-685)
- `filter_icon` (lines 1005-1018)
- `filter_color` (lines 1020-1034)

**Imports:** `crate::model::ProcessInfo`, `gpui::*`, `gpui_component::{ActiveTheme, IconName}`

**Lines:** ~125

### 13. `src/tabs/processes/chips.rs` (NEW)

**Contains:**
- `render_filter_chip` (lines 1036-1129)
- `get_state_item` (lines 1131-1159)

**Imports:**
```rust
use crate::model::*;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{tag::Tag, menu::PopupMenuItem, ActiveTheme, Icon, IconName};
use std::rc::Rc;
use std::cell::RefCell;
use super::state::ViewState;
use super::theme::{filter_icon, filter_color};
use super::ProcessesTab;
```

**Lines:** ~165

### 14. `src/tabs/processes/table.rs` (NEW)

**Contains:**
- `impl TableDelegate for ProcessTableDelegate { columns_count, rows_count, column, render_th, render_td, perform_sort, context_menu, render_tr, render_empty }` (lines 687-1003)

**Imports:**
```rust
use crate::model::*;
use bytesize::ByteSize;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    table::{Column, ColumnFixed, ColumnSort, TableDelegate, TableState},
    ActiveTheme, Icon, IconName, Sizable,
};
use super::ProcessTableDelegate;  // need to import from delegate.rs
use super::state::ViewState;
use super::theme::{state_info, tag_icons, theme_dark_or_light};
```

**Lines:** ~325

### 15. `src/tabs/processes/render.rs` (NEW)

**Contains:**
- `impl ProcessesTab { render_toolbar, render_pid_breadcrumb, render_status_bar, render_table }` — helper functions extracted from the existing `render` method
- `impl Render for ProcessesTab { fn render(...) }` (lines 1161-1586 refactored to call helpers)

**Render helper breakdown:**
- `render_toolbar()`: search input, AND/OR button, Resource View button, State dropdown, Pick button, filter chips row (lines 1304-1452)
- `render_pid_breadcrumb()`: the PID filter breadcrumb row (lines 1478-1552)
- `render_status_bar()`: the bottom bar showing "X of Y processes, N filters"  (lines 1561-1583)  
- `render_table_component()`: the data table / skeleton (lines 1554-1560)

The `fn render` method becomes a thin orchestrator (~50 lines):
```rust
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    // ... pick_result handling, table_state/input_state init ...
    // ... needs_clear_input, needs_set_input, needs_focus_input handling ...
    div()
        .size_full()
        .flex().flex_col()
        .bg(cx.theme().background)
        .child(self.render_toolbar(window, cx))
        .child(self.render_table_component(window, cx))
        .child(self.render_status_bar(window, cx))
        .into_any_element()
}
```

**Lines:** ~450

### 16. `src/tabs/processes/tests.rs` (NEW)

**Contains:** The entire `#[cfg(test)] mod tests` block from `processes.rs` (lines 1588-1867).

**Import changes:**
- `use super::fuzzy::best_fuzzy_score;` (was `use super::best_fuzzy_score;`)
- `use crate::model::*;` (unchanged)

**Lines:** ~280

### 17. `src/tabs/settings/mod.rs` (from `src/tabs/settings.rs`)

**Contains:**
- `pub struct SettingsTab`
- `impl SettingsTab { new, save, setting_pages }`
- `impl Render for SettingsTab { fn render(...) }`

**Lines:** ~80 (was 427)

### 18. `src/tabs/settings/general.rs` (NEW)

**Contains:** Free function `pub fn general_page(view, refresh_ms, theme_cell, default_config) -> SettingPage`

**Lines:** ~105 (extracted from lines 58-161 of settings.rs)

### 19. `src/tabs/settings/processes.rs` (NEW)

**Contains:** Free function `pub fn processes_page(view, default_config) -> SettingPage`

**Lines:** ~150 (extracted from lines 163-315 of settings.rs)

### 20. `src/tabs/settings/columns.rs` (NEW)

**Contains:** Free function `pub fn columns_page(view, default_config) -> SettingPage`

**Lines:** ~75 (extracted from lines 317-377 of settings.rs)

### 21. `src/tabs/settings/about.rs` (NEW)

**Contains:** Free function `pub fn about_page() -> SettingPage`

**Lines:** ~40 (extracted from lines 379-416 of settings.rs)

### 22. `src/app.rs`

**Import changes only:**
```rust
// Before:
use crate::collect::{detect_gpu, SystemCollector};

// After:
use crate::platform::{detect_gpu, SystemCollector};
```

**Lines:** 254 (unchanged, import line only changes)

### 23. `src/collect.rs` & `src/wayland.rs`

**Deleted.** All moved to `src/platform/linux/`.

---

## Import Dependency Graph After Restructure

```
app.rs
  ├── platform::{detect_gpu, SystemCollector}
  ├── config::Config
  ├── model::{GpuBackend, SystemSnapshot, Theme}
  └── tabs::{PerformanceTab, ProcessesTab, SettingsTab}

platform/mod.rs     → re-exports from platform/linux/
platform/linux/mod.rs
  ├── collector.rs  (SystemCollector)
  ├── gpu.rs        (detect_gpu, build_vram_map)
  ├── process.rs    (impl SystemCollector)
  │     └── uses gpu::build_vram_map
  └── wayland.rs    (get_focused_window_app_id, get_toplevels)

tabs/processes/
  ├── mod.rs        → ProcessesTab
  ├── state.rs      → CumulativeResources, ViewState
  ├── delegate.rs   → ProcessTableDelegate
  │     ├── uses state::ViewState
  │     └── uses fuzzy::{fuzzy_match, best_fuzzy_score}
  ├── fuzzy.rs      → fuzzy_match, nucleo_fuzzy_score, best_fuzzy_score
  ├── theme.rs      → TagType, theme helpers
  ├── chips.rs      → render_filter_chip, get_state_item
  │     ├── uses theme::{filter_icon, filter_color}
  │     └── uses state::ViewState
  ├── table.rs      → TableDelegate impl
  │     ├── uses delegate::ProcessTableDelegate
  │     └── uses theme::{state_info, tag_icons, theme_dark_or_light}
  ├── render.rs     → Render impl
  │     └── uses chips::{render_filter_chip, get_state_item}
  └── tests.rs      → tests
        └── uses fuzzy::best_fuzzy_score

tabs/settings/
  ├── mod.rs        → SettingsTab
  ├── general.rs    → general_page()
  ├── processes.rs  → processes_page()
  ├── columns.rs    → columns_page()
  └── about.rs      → about_page()

widgets/
  └── process_context.rs → build_process_menu()
```

---

## Implementation Steps (in order)

1. **Create directory structure:**
   ```
   mkdir -p src/platform/linux
   mkdir -p src/tabs/settings
   mkdir -p src/tabs/processes
   ```

2. **Move `src/wayland.rs` → `src/platform/linux/wayland.rs`**
   - All imports in wayland.rs are self-contained (wayland_client, std).
   - Reference to `crate::GPUITOP_APP_ID` stays the same.

3. **Split `src/collect.rs` into `src/platform/linux/{collector,process,gpu}.rs`**
   - Create `collector.rs`: struct def + CPU/mem/disk/net methods
   - Create `process.rs`: `collect_processes` + electron helpers + tests
   - Create `gpu.rs`: VRAM maps + detect_gpu

4. **Create `src/platform/mod.rs` and `src/platform/linux/mod.rs`**

5. **Update `src/main.rs`:** replace `mod wayland; mod collect;` with `mod platform;`

6. **Update `src/app.rs`:** change `crate::collect::*` to `crate::platform::*`

7. **Update `src/tabs/processes.rs` wayland import:**
   - Change `crate::wayland::get_focused_window_app_id` to `crate::platform::get_focused_window_app_id`

8. **Split `src/tabs/processes.rs` into the 9 files** under `src/tabs/processes/`

9. **Split `src/tabs/settings.rs` into the 5 files** under `src/tabs/settings/`

10. **Update `src/tabs/mod.rs`:** change `pub mod processes; pub mod settings;` → `pub mod processes; pub mod settings;` (module declarations change from file-based to directory-based — Rust resolves this automatically: if `processes.rs` is replaced by `processes/mod.rs`, the `pub mod processes;` declaration still works)

11. **Delete old files:** `src/collect.rs`, `src/wayland.rs`, `src/tabs/processes.rs`, `src/tabs/settings.rs`

12. **Build, test, verify:** `cargo check && cargo test`

---

## File Size Summary

| File | Lines (approx) |
|---|---|
| `src/main.rs` | 61 |
| `src/app.rs` | 254 |
| `src/model.rs` | 361 |
| `src/config.rs` | 158 |
| `src/platform/mod.rs` | 8 |
| `src/platform/linux/mod.rs` | 10 |
| `src/platform/linux/collector.rs` | 400 |
| `src/platform/linux/process.rs` | 400 |
| `src/platform/linux/gpu.rs` | 130 |
| `src/platform/linux/wayland.rs` | 196 |
| `src/tabs/mod.rs` | 14 |
| `src/tabs/performance.rs` | 270 |
| `src/tabs/settings/mod.rs` | 80 |
| `src/tabs/settings/general.rs` | 105 |
| `src/tabs/settings/processes.rs` | 150 |
| `src/tabs/settings/columns.rs` | 75 |
| `src/tabs/settings/about.rs` | 40 |
| `src/tabs/processes/mod.rs` | 120 |
| `src/tabs/processes/state.rs` | 55 |
| `src/tabs/processes/delegate.rs` | 420 |
| `src/tabs/processes/fuzzy.rs` | 45 |
| `src/tabs/processes/theme.rs` | 125 |
| `src/tabs/processes/chips.rs` | 165 |
| `src/tabs/processes/table.rs` | 325 |
| `src/tabs/processes/render.rs` | 450 |
| `src/tabs/processes/tests.rs` | 280 |
| `src/widgets/mod.rs` | 1 |
| `src/widgets/process_context.rs` | 109 |
| **TOTAL** | **4,639** |

Largest file: `src/tabs/processes/render.rs` at ~450 lines (was 1,867).

No single module exceeds 500 lines. Platform-specific code is cleanly isolated
in `platform/linux/`. Adding Windows/macOS support later means creating a parallel
module tree under `platform/windows/` or `platform/macos/` with the same public API.
