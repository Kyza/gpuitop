## Build & Test

```bash
cargo check                               # Fast compile check
cargo test                                # Run all 249 tests
cargo test -- <test_name>                 # Single test
cargo build --release                     # Release (LTO thin, strip=symbols)
```

**Always format after finishing changes AND before committing:**
```bash
cargo +nightly fmt
```

Build/check in debug mode unless running `cargo test` — release builds take forever due to git-pulled gpui deps plus `opt-level = 2` for `[profile.dev.package."*"]`.

## Features

```toml
default = []
hotpath = ["hotpath/hotpath"]
hotpath-cpu = ["hotpath/hotpath-cpu"]
hotpath-alloc = ["hotpath/hotpath-alloc"]
hotpath-mcp = ["hotpath/hotpath-mcp"]
```

## Architecture

```
src/
├── data/                     # ZERO gpui/gpui_component imports. Pure Rust logic.
│   ├── model.rs              # ProcessSnapshot, SystemSnapshot, Filter, SortColumn, enums
│   ├── config.rs             # Config, load/save (RON format)
│   ├── state.rs              # ViewState, CumulativeResources
│   ├── fuzzy.rs              # nucleo fuzzy matching
│   ├── settings.rs           # Settings mutations — no UI types
│   │
│   ├── snapshot/             # System tick + process collection
│   │   ├── mod.rs            # cfg dispatch
│   │   ├── linux/            # CollectorState, collect_*, SystemSnapshot::new(), electron
│   │   ├── windows.rs        # stub
│   │   └── macos.rs          # stub
│   │
│   ├── gpu/                  # GPU detection (nvml, AMD ROCm)
│   │   ├── mod.rs            # cfg dispatch
│   │   ├── linux.rs          # nvml + AMD, build_vram_map
│   │   ├── windows.rs        # stub
│   │   └── macos.rs          # stub
│   │
│   ├── icons/                # Desktop entry icon caching
│   │   ├── mod.rs            # cfg dispatch
│   │   ├── linux.rs          # parse .desktop files, resolve icon paths
│   │   ├── windows.rs        # stub
│   │   └── macos.rs          # stub
│   │
│   ├── service_manager/      # Init system / service detection
│   │   ├── mod.rs            # InitSystem enum, cfg dispatch
│   │   ├── linux.rs          # systemd/openrc/runit/dinit/sysv detection
│   │   ├── windows.rs        # stub
│   │   └── macos.rs          # stub
│   │
│   ├── window_picker/        # Active window detection
│   │   ├── mod.rs            # cfg dispatch + is_window_picker_available
│   │   ├── linux/
│   │   │   ├── mod.rs        # runtime detection via WAYLAND_DISPLAY
│   │   │   ├── wayland.rs    # Wayland foreign-toplevel focus
│   │   │   └── x11.rs        # X11 stub
│   │   ├── windows.rs        # stub
│   │   │   └── macos.rs      # stub
│   │
│   ├── properties/            # Per-process detail collection (procfs)
│   │   ├── mod.rs            # ProcessProperties struct + cfg dispatch
│   │   ├── linux.rs          # collect(pid) via procfs
│   │   ├── windows.rs        # stub
│   │   └── macos.rs          # stub
│   │
│   └── processes/
│       ├── delegate.rs       # ProcessTableDelegate (filtering, sorting, cumulative)
│       ├── tree.rs           # Tree helpers (with_ancestors, ancestors_to_expand)
│       └── tests.rs          # Standalone unit tests (cfg(test))
│
├── ui/                       # ALL gpui rendering. Depends on data/. Never reverse.
│   ├── theme.rs              # Tag colors, icons (returns Hsla/IconName)
│   ├── app/
│   │   └── app_view.rs      # App struct, Render impl, background-collector thread
│   ├── processes/
│   │   ├── tab.rs            # ProcessesTab struct, subscriptions
│   │   ├── table_delegate.rs # impl TableDelegate (data→UI bridge) + column rendering
│   │   ├── toolbar.rs        # Search input, filter toggles
│   │   ├── breadcrumbs.rs    # Pinned-PID breadcrumb row
│   │   ├── status_bar.rs     # "Showing X of Y processes"
│   │   ├── list_view.rs      # DataTable wrapper
│   │   ├── tree_view.rs      # Tree widget + per-node content
│   │   ├── chips.rs          # Filter chip renders
│   │   └── context_menu.rs   # Right-click process menu
│   ├── performance/
│   │   ├── tab.rs            # PerformanceTab, Render
│   │   ├── cpu.rs            # CPU gauges
│   │   ├── disk.rs           # Disk I/O
│   │   ├── network.rs        # Network I/O
│   │   └── widgets.rs        # Shared UI helpers
│   ├── properties_window/    # Process detail popup window
│   │   ├── mod.rs            # PropertiesWindow struct + tick loop + cfg dispatch
│   │   ├── linux.rs          # Linux body — tabbed DescriptionList
│   │   ├── windows.rs        # stub
│   │   └── macos.rs          # stub
│   └── settings/
│       ├── tab.rs            # SettingsTab, Render
│       ├── general.rs        # General page (UI only)
│       ├── processes.rs      # Processes page (UI only)
│       └── about.rs          # About page
│
└── main.rs                   # Thin bootstrap (window creation, app launch)
```

### Hard rules

1. **`data/` compiles without gpui.** If a file imports `gpui::*`, `gpui::prelude::*`, or `gpui_component::*`, it does not belong in `data/`.
2. **`data/` never imports from `ui/`.** Dependencies flow one way: `ui/ → data/`.
3. **Settings mutations in `data/settings.rs`.** `SettingsTab` in `ui/` calls into data methods; UI pages only build `SettingPage`/`SettingField` widgets and wire callbacks.
4. **`ProcessTableDelegate` struct in `data/processes/delegate.rs`.** The `TableDelegate` trait impl (gpui trait) lives in `ui/processes/table_delegate.rs`. Rust allows trait impls across modules — the struct stays data-pure.
5. **Per-feature platform dispatch.** Each feature (gpu, icons, service_manager, snapshot, window_picker, properties_window) has its own module with `mod.rs` doing `#[cfg(target_os = "...")]` dispatch. Platform implementations live in sub-modules (`linux.rs`, `windows.rs`, `macos.rs`). The `window_picker` module adds runtime compositor dispatch on Linux (`linux/mod.rs` checks `WAYLAND_DISPLAY`).
6. **`CollectionState` + `SystemSnapshot::new()` pattern.** The platform module provides a `CollectorState` struct (mutable tick state) and `impl SystemSnapshot { pub fn new(state: &mut CollectorState) -> Self }`. The background thread creates state once, calls `SystemSnapshot::new(&mut state)` each tick.

## Gotchas

- **`set_items()` wipes expand state.** Creates fresh `TreeItem` objects per call. Preserve expand state externally via `TreeData::preserve_expand_from` — walks old cached items by PID-based ID and re-applies `.expanded(true)` on new items before `set_items()`.
- **Collector runs on background thread.** `SystemSnapshot::new(&mut CollectorState)` looped via `mpsc::channel`. Snapshots drained in `App::render` each frame. UI never reads /proc directly.
- **Cumulative cache** (`cum_cache`) computed once per snapshot, reused by filtering and rendering. Invalidate by setting to `None` on new snapshot.
- **`[profile.dev.package."*"]` opt-level = 2** — dependencies optimized even in debug. Startup fast, incremental `cargo check` still fast.
- **`procfs` crate** — all Linux /proc reads go through `procfs` (0.16). No raw `fs::read_to_string("/proc/...")`. `Meminfo::current()`, `KernelStats::current()`, `diskstats()`, `net::dev_status()`, `Process::stat()`, `Process::status()`, `Process::io()`, `Process::cmdline()`, `Process::cgroups()`, `Process::environ()`, `Process::exe()`, `Process::cwd()`, `Process::fd()`, `Process::limits()`.
- **`target_os = "macos"`** — not `"darwin"`. Rust uses `macos` for the target triple. Module files are named `macos.rs` accordingly.

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
