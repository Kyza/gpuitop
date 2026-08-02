## Build & Test

```bash
cargo check                               # Fast compile check
cargo test                                # Run all 36 tests
cargo test -- <test_name>                 # Single test
cargo build --release                     # Release (LTO thin, strip=symbols)
```

**Always format after finishing changes AND before committing:**
```bash
cargo +nightly fmt
```

Build/check in debug mode unless running `cargo test` — release builds take forever due to git-pulled gpui deps plus `opt-level = 2` for `[profile.dev.package."*"]`.

## Architecture

```
src/
├── data/              # ZERO gpui/gpui_component imports. Pure Rust logic.
│   ├── model.rs       # ProcessInfo, Filter, SortColumn, enums
│   ├── config.rs      # Config, load/save (RON format)
│   ├── state.rs       # ViewState, CumulativeResources
│   ├── fuzzy.rs       # nucleo fuzzy matching
│   ├── settings.rs    # Settings mutations — no UI types
│   ├── platform/      # /proc, nvml, Wayland (already clean)
│   ├── processes/
│   │   ├── delegate.rs    # ProcessTableDelegate struct
│   │   ├── filter.rs      # proc_matches, descendant lookup
│   │   ├── sort.rs        # Sorting/comparison
│   │   ├── cumulative.rs  # Compute aggregate cumulative map
│   │   ├── tree.rs        # build_tree, TreeData
│   │   └── tests.rs       # Unit tests (cfg(test))
│
├── ui/                # ALL gpui rendering. Depends on data/. Never reverse.
│   ├── theme.rs       # Tag colors, icons (returns Hsla/IconName)
│   ├── app/
│   │   ├── app_view.rs    # App struct, Render impl
│   │   └── root.rs        # Window setup
│   ├── processes/
│   │   ├── tab.rs         # ProcessesTab struct, subscriptions
│   │   ├── table_delegate.rs  # impl TableDelegate (data→UI bridge)
│   │   ├── columns.rs     # Per-column cell rendering
│   │   ├── toolbar.rs     # Search input, filter toggles
│   │   ├── breadcrumbs.rs # Pinned-PID breadcrumb row
│   │   ├── status_bar.rs  # "Showing X of Y processes"
│   │   ├── list_view.rs   # DataTable wrapper
│   │   ├── tree_view.rs   # Tree widget + per-node content
│   │   ├── chips.rs       # Filter chip renders
│   │   └── context_menu.rs # Right-click process menu
│   ├── performance/
│   │   ├── tab.rs         # PerformanceTab, Render
│   │   ├── cpu.rs         # CPU gauges
│   │   ├── disk.rs        # Disk I/O
│   │   ├── network.rs     # Network I/O
│   │   └── widgets.rs     # Shared UI helpers
│   └── settings/
│       ├── tab.rs         # SettingsTab, Render
│       ├── general.rs     # General page (UI only)
│       ├── processes.rs   # Processes page (UI only)
│       └── about.rs       # About page
│
└── main.rs             # Thin bootstrap (window creation, app launch)
```

### Hard rules

1. **`data/` compiles without gpui.** If a file imports `gpui::*`, `gpui::prelude::*`, or `gpui_component::*`, it does not belong in `data/`.
2. **`data/` never imports from `ui/`.** Dependencies flow one way: `ui/ → data/`.
3. **Settings mutations in `data/settings.rs`.** `SettingsTab` in `ui/` calls into data methods; UI pages only build `SettingPage`/`SettingField` widgets and wire callbacks.
4. **`ProcessTableDelegate` struct in `data/processes/delegate.rs`.** The `TableDelegate` trait impl (gpui trait) lives in `ui/processes/table_delegate.rs`. Rust allows trait impls across modules — the struct stays data-pure.

## Gotchas

- **`set_items()` wipes expand state.** Creates fresh `TreeItem` objects per call. Preserve expand state externally via `TreeData::preserve_expand_from` — walks old cached items by PID-based ID and re-applies `.expanded(true)` on new items before `set_items()`.
- **Collector runs on background thread.** `SystemCollector::tick()` looped via `mpsc::channel`. Snapshots drained in `App::render` each frame. UI never reads /proc directly.
- **Cumulative cache** (`cum_cache`) computed once per snapshot, reused by filtering and rendering. Invalidate by setting to `None` on new snapshot.
- **`[profile.dev.package."*"]` opt-level = 2** — dependencies optimized even in debug. Startup fast, incremental `cargo check` still fast.

## Committing

- **NEVER commit or push without explicit permission.** Do not stage files, create commits, or push branches unless the user explicitly tells you to.
- **NEVER amend commits.**
- **NEVER force-push** unless the user explicitly tells you to.

## Style

```toml
hard_tabs = true
max_width = 78
format_strings = true
format_macro_bodies = true
format_code_in_doc_comments = true
format_macro_matchers = true
```

- Tabs for indentation, 78 column max.
- No linter beyond rustfmt.
- No comments unless explicitly requested.
- **Files should never be thousands of lines long.** Target ~200-300 lines per file. If a file exceeds ~400 lines, split into separate concepts: extract helper functions, split related behavior into sub-modules, or componentize UI into smaller elements. Rarely go beyond 500 lines.
- **Write unit tests for data/ modules** when the logic is testable (filtering, sorting, fuzzy matching, cumulative computation, settings mutations). UI files may also have tests for pure helper functions extracted from rendering.
