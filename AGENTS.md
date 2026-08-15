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

`hotpath` is declared per-crate and forwarded down the dependency chain. Each
crate that uses `#[hotpath::measure]` (core, gpu, snapshot, ui) declares:

```toml
default = []
hotpath = ["hotpath/hotpath"]
hotpath-cpu = ["hotpath/hotpath-cpu"]
hotpath-alloc = ["hotpath/hotpath-alloc"]
hotpath-mcp = ["hotpath/hotpath-mcp"]
```

`gpuitop-ui` forwards to its data-crate deps, and the `gpuitop` bin re-exposes
the flags while forwarding to `gpuitop-core` + `gpuitop-ui` (only direct deps
can be feature-gated):

```toml
# gpuitop-ui
hotpath = ["hotpath/hotpath", "gpuitop-core/hotpath", "gpuitop-gpu/hotpath", "gpuitop-snapshot/hotpath"]
# gpuitop bin
hotpath = ["hotpath/hotpath", "gpuitop-core/hotpath", "gpuitop-ui/hotpath"]
```

## Architecture

```
crates/                        # virtual workspace (root Cargo.toml has no [package])
├── gpuitop/                   # binary — thin bootstrap (main.rs, cli.rs)
├── core/                      # gpuitop-core — ZERO gpui/gpui_component. Pure logic.
│   ├── model.rs               # ProcessSnapshot, SystemSnapshot, Filter, SortColumn, enums
│   ├── config.rs              # Config, load/save (RON format)
│   ├── state.rs               # ViewState, CumulativeResources
│   ├── fuzzy.rs               # nucleo fuzzy matching
│   ├── settings.rs            # Settings mutations — no UI types
│   ├── merge.rs               # ron2 deep_merge
│   ├── service_manager/       # InitSystem enum + oswap dispatch (detect_init, is_service, …)
│   └── processes/             # ProcessTableDelegate (filtering, sorting, cumulative), tree helpers
├── gpu/                       # gpuitop-gpu — nvml/ROCm detection via oswap
├── icons/                     # gpuitop-icons — DesktopEntryCache + icon path resolution via oswap
├── snapshot/                  # gpuitop-snapshot — CollectorState + collect_snapshot via oswap
├── properties/                # gpuitop-properties — ProcessProperties collect(pid) via oswap
├── window-picker/             # gpuitop-window-picker — PickedWindow, GPUITOP_APP_ID via oswap
└── ui/                        # gpuitop-ui — ALL gpui rendering. Depends on the data crates.
    ├── themes.rs              # built-in theme load/apply (gpui-component + rust-embed)
    ├── theme.rs               # Tag colors, icons
    ├── app/app_view.rs        # App struct, Render impl, background-collector thread
    ├── processes/             # tab, table_delegate, delegate (newtype), tree, toolbar, …
    ├── performance/           # PerformanceTab
    ├── properties_window/     # Process detail popup window
    └── settings/              # SettingsTab + General/Processes/About pages
```

Dependency direction is one-way: `ui → {snapshot, properties, window-picker,
gpu, icons} → core`, plus `bin → {ui, icons, window-picker, core}`.

### Hard rules

1. **Data crates compile without gpui.** `core`, `gpu`, `icons`, `snapshot`, `properties`, and `window-picker` must not import `gpui::*`, `gpui::prelude::*`, or `gpui_component::*`. Only `ui` (and the bin) may depend on gpui.
2. **No reverse imports.** Data crates never import from `ui`. The bin may import from both.
3. **Settings mutations in `core/settings.rs`.** `SettingsTab` in `ui/` calls into core methods; UI pages only build `SettingPage`/`SettingField` widgets and wire callbacks.
4. **`ProcessTableDelegate` struct stays in `core/processes/delegate.rs` (data-pure).** The `TableDelegate` impl and `build_tree` live on a newtype in `ui/processes/delegate.rs` (`pub struct ProcessTableDelegate(pub CoreDelegate)` + `Deref`/`DerefMut`) — the orphan rule forbids impl'ing a foreign trait (gpui_component's `TableDelegate`) for a foreign type.
5. **Per-feature platform dispatch via `oswap`.** Each platform feature (gpu, icons, service_manager, snapshot, window_picker, properties) calls `define_interface!` (marker struct + trait + re-exported free fns) and `define_platforms!` (cfg-gated `linux.rs`/`macos.rs`/`windows.rs`). Platform files implement the trait with `impl_interface!`. Keep the marker struct crate-private (bare `Platform`) so `impl_interface!` is not `#[macro_export]`ed.
6. **`CollectorState` + `collect_snapshot` pattern.** `CollectorState` (mutable tick state) lives in `snapshot/src/lib.rs`. The interface exposes `collect_snapshot(&mut CollectorState) -> SystemSnapshot` (a free function, not an inherent `SystemSnapshot::new`). The background thread creates state once and calls `collect_snapshot(&mut state)` each tick.

## Gotchas

- **`set_items()` wipes expand state.** Creates fresh `TreeItem` objects per call. Preserve expand state externally via `TreeData::preserve_expand_from` — walks old cached items by PID-based ID and re-applies `.expanded(true)` on new items before `set_items()`.
- **Collector runs on background thread.** `collect_snapshot(&mut CollectorState)` looped via `mpsc::channel`. Snapshots drained in `App::render` each frame. UI never reads /proc directly.
- **`oswap` interfaces functions only.** Shared types (`CollectorState`, `DesktopEntryCache`) live in the crate's `lib.rs`; constructors/associated fns become interface fns (`collect_snapshot`, `load_cache`). `define_platforms!` emits single-file modules (`linux.rs`), so directory-based platform code (snapshot, window-picker) uses a `linux.rs` entry that declares `#[path = "linux/*.rs"]` submodules.
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
- **Write unit tests for data crates** when the logic is testable (filtering, sorting, fuzzy matching, cumulative computation, settings mutations). UI files may also have tests for pure helper functions extracted from rendering.
