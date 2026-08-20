## Build & Test

```bash
cargo check                               # Fast compile check
cargo test                                # Run all 250 tests
cargo test -- <test_name>                 # Single test
cargo build --release                     # Release (LTO thin, strip=symbols)
```

**Always format after finishing changes AND before committing:**
```bash
cargo +nightly fmt
```

Build/check in debug mode unless running `cargo test` — release builds take forever due to git-pulled gpui deps plus `opt-level = 2` for `[profile.dev.package."*"]`.

When piping long-running builds/tests, include the cargo progress lines in the filter so you can tell the build is moving and not stuck:

```bash
cargo test 2>&1 | rg "Building|Compiling|Running|test result:"
```

## Features

`hotpath` is declared per-crate and forwarded down the dependency chain. Each
crate that uses `#[hotpath::measure]` (core, gpu, snapshot, processes) declares:

```toml
default = []
hotpath = ["hotpath/hotpath"]
hotpath-cpu = ["hotpath/hotpath-cpu"]
hotpath-alloc = ["hotpath/hotpath-alloc"]
hotpath-mcp = ["hotpath/hotpath-mcp"]
```

The `gpuitop` bin re-exposes the flags while forwarding to the crates that use
`#[hotpath::measure]` (only direct deps can be feature-gated):

```toml
# gpuitop bin
hotpath = [
	"hotpath/hotpath",
	"gpuitop_core/hotpath",
	"gpuitop_gpu/hotpath",
	"gpuitop_snapshot/hotpath",
	"gpuitop_processes/hotpath",
]
```

## Architecture

```
crates/                        # virtual workspace (root Cargo.toml has no [package])
├── gpuitop/                   # binary — bootstrap + app shell (main.rs, cli.rs, app_view.rs)
├── core/                      # gpuitop_core — ZERO gpui/gpui_component. Pure logic.
│   ├── model.rs               # ProcessSnapshot, SystemSnapshot, Filter, SortColumn, enums
│   ├── config.rs              # Config, load/save (RON format)
│   ├── state.rs               # ViewState, CumulativeResources
│   ├── fuzzy.rs               # nucleo fuzzy matching
│   ├── merge.rs               # ron2 deep_merge
│   ├── service_manager/       # InitSystem enum + cfg dispatch (detect_init, is_service, …)
│   └── processes/             # ProcessTableDelegate (filtering, sorting, cumulative), tree helpers
├── gpu/                       # gpuitop_gpu — nvml/ROCm detection via cfg
├── icons/                     # gpuitop_icons — DesktopEntryCache + icon path resolution via cfg
├── snapshot/                  # gpuitop_snapshot — CollectorState + collect_snapshot via cfg
├── window_picker/             # gpuitop_window_picker — PickedWindow, GPUITOP_APP_ID via cfg
├── components/                # gpuitop_components — shared UI building blocks
│   ├── selectable_text.rs     # SelectableText — verbatim text that is selectable/copyable
│   ├── themes.rs              # built-in theme load/apply (gpui-component + rust-embed)
│   ├── theme.rs               # Tag colors, state/filter icons + colors
│   ├── theme_menu.rs          # build_theme_menu (shared by titlebar + settings)
│   └── assets/                # lucide (LucideIcon), layered (LayeredAssets)
├── properties/                # gpuitop_properties — ProcessProperties collect(pid) + PropertiesWindow UI
│   └── window/                # Process detail popup window
├── processes/                 # gpuitop_processes — ProcessesTab + table/tree/toolbar UI
├── performance/               # gpuitop_performance — PerformanceTab
└── settings/                  # gpuitop_settings — SettingsTab + General/Processes/About pages
    └── mutations.rs           # Settings mutations (moved from core/settings.rs)
```

Dependency direction is one-way: panels → components → core, with data crates
(`gpu`, `icons`, `snapshot`, `window_picker`) → core. The `gpuitop` bin depends
on everything and hosts the `App` shell (`app_view.rs`).

### Hard rules

1. **Data crates compile without gpui.** `core`, `gpu`, `icons`, `snapshot`, and `window_picker` must not import `gpui::*`, `gpui::prelude::*`, or `gpui_component::*`. Only `components`, `processes`, `performance`, `settings`, `properties` (which now hosts the UI window), and the bin may depend on gpui.
2. **No reverse imports.** Data crates never import from panel/component crates. The bin may import from all.
3. **Settings mutations in `settings/mutations.rs`.** `SettingsTab` calls into those methods; UI pages only build `SettingPage`/`SettingField` widgets and wire callbacks.
4. **`ProcessEngine` is the process-pipeline home; the UI newtype wraps it directly.** All filtering/sorting/cumulative logic lives on `ProcessEngine` in `core/processes/engine.rs`. The gpui side uses a newtype in `processes/delegate.rs` (`pub struct ProcessTableDelegate(pub Rc<RefCell<ProcessEngine>>)` + `Deref`/`DerefMut`) so the orphan rule can be satisfied — gpui_component's `TableDelegate` impl and `build_tree` live there, forwarding to the engine.
5. **Per-feature platform dispatch via `#[cfg(target_os)]` modules.** Each platform feature (gpu, icons, service_manager, snapshot, window_picker, properties) declares `#[cfg(target_os)] mod linux/macos/windows;` and `pub use`s the free functions from the matching platform. Platform files (`linux.rs`, `macos.rs`, `windows.rs`) define plain `pub fn`s; shared types live in the crate's `lib.rs`.
6. **`CollectorState` + `collect_snapshot` pattern.** `CollectorState` (mutable tick state) lives in `snapshot/src/lib.rs`. The interface exposes `collect_snapshot(&mut CollectorState) -> SystemSnapshot` (a free function, not an inherent `SystemSnapshot::new`). The background thread creates state once and calls `collect_snapshot(&mut state)` each tick.

## Gotchas

- **`set_items()` wipes expand state.** Creates fresh `TreeItem` objects per call. Preserve expand state externally via `TreeData::preserve_expand_from` — walks old cached items by PID-based ID and re-applies `.expanded(true)` on new items before `set_items()`.
- **Collector runs on background thread.** `collect_snapshot(&mut CollectorState)` looped via `mpsc::channel`. Snapshots drained in `App::render` each frame. UI never reads /proc directly.
- **`#[cfg(target_os)]` interfaces functions only.** Shared types (`CollectorState`, `DesktopEntryCache`) live in the crate's `lib.rs`; constructors/associated fns become free fns (`collect_snapshot`, `load_cache`) re-exported from the platform modules. Directory-based platform code (snapshot, window_picker) uses a `linux.rs` entry that declares `#[path = "linux/*.rs"]` submodules.
- **Cumulative cache** (`cum_cache`) computed once per snapshot, reused by filtering and rendering. Invalidate by setting to `None` on new snapshot.
- **`[profile.dev.package."*"]` opt-level = 2** — dependencies optimized even in debug. Startup fast, incremental `cargo check` still fast.
- **`procfs` crate** — all Linux /proc reads go through `procfs` (0.16). No raw `fs::read_to_string("/proc/...")`. `Meminfo::current()`, `KernelStats::current()`, `diskstats()`, `net::dev_status()`, `Process::stat()`, `Process::status()`, `Process::io()`, `Process::cmdline()`, `Process::cgroups()`, `Process::environ()`, `Process::exe()`, `Process::cwd()`, `Process::fd()`, `Process::limits()`.
- **`target_os = "macos"`** — not `"darwin"`. Rust uses `macos` for the target triple. Module files are named `macos.rs` accordingly.
- **Selectable text = `SelectableText`** (`components/selectable_text.rs`). gpui has no built-in selectable text, and gpui-component's `TextView` only parses Markdown/HTML (so it mangles values containing markup). `SelectableText` HTML-escapes the input and wraps `TextView::html`, so text renders verbatim while selection/copy stays exact. Each instance needs a stable unique id.
- **`settings/about.rs` uses `env!("CARGO_PKG_*")`** — those resolve to the *settings* crate's metadata, so `settings/Cargo.toml` must mirror the bin's `description`/`authors`/`version` or the About page shows empty strings.
- **About-page dependency list comes from the bin.** `gpuitop/build.rs` emits `built.rs` (`DIRECT_DEPS: &[gpuitop_core::about::DepInfo]`) from the *bin's* direct deps, `main.rs` `include!`s it, and `app_view` passes it into `SettingsTab::new`. Keep `build.rs` in the bin — the `DepInfo` type lives in `core/src/about.rs` so both sides share it.

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

## Agent skills

### Questions

Always present questions to the user (grilling rounds, design decisions, any
decision point) with the ui-based question tool — never as plain-text lists.

### Issue tracker

Issues and specs live as markdown files under `.scratch/<feature>/` in this repo. See `docs/agents/issue-tracker.md`.

### Triage labels

The five canonical triage roles use the default label strings (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Multi-context: root `CONTEXT-MAP.md` points at one `CONTEXT.md` per crate, with ADRs under `docs/adr/`. See `docs/agents/domain.md`.
