# Core

The data model and pure logic of gpuitop: what a system looks like at one instant, how it is filtered and sorted, and what the user's view and config state are. Contains zero UI — gpui and gpui_component are not imported here.

## Language

### The system model

**SystemSnapshot**:
An immutable picture of the whole machine at one collection tick: all processes plus CPU, memory, disks, networks, GPU, and a timestamp. `gpu_polling_enabled` records whether the GPU reads actually ran this tick, so the UI can distinguish "GPU data is off" from "no GPU detected".
_Avoid_: "tick data", "frame"

**ProcessSnapshot**:
An immutable record of one process at one tick: pid/ppid, name, user, state, command, resource usage, and the classification flags (`is_gui`, `is_kthread`, `is_owned_by_current_user`, `is_electron`, `has_children`).
_Avoid_: "process row", "proc"

**CpuInfo**:
Aggregate CPU state: per-core `CpuCore`s, overall usage, model name, temperature.
**MemoryInfo**:
System memory in bytes: total, used, available, cached, swap used.
**DiskInfo**:
One disk's read/write bytes-per-second.
**NetInfo**:
One network interface's rx/tx bytes-per-second.

### Process classification

**GUI process**:
A process with a `DISPLAY` or `WAYLAND_DISPLAY` environment variable; surfaced as the `is_gui` flag and the `Gui` filter.
_Avoid_: "display process"

**Kernel thread**:
A process whose parent is `kthreadd` (ppid 2) or whose name is bracketed (`[foo]`); skips expensive `/proc` reads during collection.
_Avoid_: "system thread"

**Electron app**:
A process belonging to an Electron application, identified by walking `--type=` argv chains to the validated root and assigned a friendly `electron_app_name`. A first-class concept, not a footnote — it participates in filtering and search ranking.
_Avoid_: "chrome subprocess", "renderer"

**Service**:
A process managed by the detected init system (systemd/OpenRC/runit/dinit/SysV); the `Services` filter. See **InitSystem**.
_Avoid_: "daemon" (a daemon is not always a service)

**Owned by current user**:
The process runs as the uid the collector started as; the `is_owned_by_current_user` flag. Note the `User` filter additionally excludes GUI apps.

### Filtering and sorting

**Filter**:
One user-applied process predicate — a type toggle (`Gui`, `User`, `System`, `Services`, `Kernel`, `Parent`, `Vram`, `Nvidia`, `Amd`, `Electron`), a state char, a username string, or a PID. Each variant is one toolbar checkbox.
_Avoid_: "category", "checkbox filter"

**FilterMode**:
How multiple `Filter`s combine: `And` (all must match) or `Or` (any must match).

**Filter::Pid / pin**:
A `Filter::Pid` restricts the list to one process and its subtree. In the UI this is the *pin*: the row is forced to the top, breadcrumbs show its ancestor chain, and search can be cleared on pin. One concept, two names.
_Avoid_: "follow" (unless naming the follow-the-pid behavior)

**System filter**:
The `System` filter means precisely `not a kernel thread && not owned by the current user && ppid != 1`. It is a computed bucket, not the loose "system processes".
_Avoid_: treating it as "everything that isn't mine"

**SortColumn**:
The nine sortable process columns (name, pid, user, state, cpu, memory, vram, disk read, disk write). Bridges to 1-based table column indices; the name column can never be hidden.
_Avoid_: "column" alone (a column is a render concern)

**SortDirection**:
Ascending or descending; applied by reversing the comparator.

**ResourceViewMode**:
Whether resource columns show a process's own usage (`SelfOnly`, default) or its subtree total (`Cumulative`). Affects displayed values, sorting, and VRAM filters simultaneously.
_Avoid_: "Self mode", "cum mode"

**PidFilterMode**:
Scope of a `Filter::Pid`: `DirectChildren` (default) or `AllDescendants`.

**ProcessGrouping**:
How the list is grouped: `Auto`, `ByUser`, `ByState`, `Flat`.
**DefaultViewMode**:
Initial layout of the list: `List` (default) or `Tree`.
**GpuData**:
Whether GPU data (per-PID VRAM attribution and per-device telemetry) is collected at all: `On` (default) or `Off`. Live-mirrored to the collector thread; Off produces zero GPU reads per tick.

### View state

**ProcessEngine**:
The shared process engine behind the process list: the snapshot (swapped only via `set_snapshot`), the config, the init system, and three private caches (rows + match set, cumulative map, the process graph). `rows()` is the single pipeline entry point — search, filters, sort, pin — cached on `ViewState.generation`. `set_snapshot` and `ViewState::mutate` are the only two invalidators. All methods take `&self`; the caches are inner `RefCell`s so callers can hold an immutable engine borrow while calling any method.
_Avoid_: "the delegate's caches" (caches live in the engine, not the handle)

**match set**:
The pipeline's filtered set: pids passing search + non-Pid filters, before any Pid scope. `match_set()` exposes it; `rows()` = `match_set ∩ list scope` (mode-aware) + sort + pin-to-top; `tree_match_set()` = `match_set ∩ full descendant scope` (the tree always shows all descendants, ignoring `pid_filter_mode`). The Pid filter is a *scope restriction*, never an Or alternative — the list and tree always show the same pids, just arranged differently.
_Avoid_: "filtered rows" for the set (rows are the sorted, scoped view of it)

**ProcessGraph**:
The process tree derived from one snapshot: parent/child structure, full subtree counts, and ancestry walks (`is_descendant_of`, `descendants_of`, `subtree_count_of`, `ancestor_chain_of`, `with_ancestors`, `ancestors_to_expand`, `display_subtree_counts`). Built lazily once per snapshot, owned by the engine, cycle-guarded so a corrupted ppid chain can never loop. The single home for every graph walk.
_Avoid_: "pid index", "descendant counts" as separate concepts (they are fields of the graph)

**ProcessTableDelegate**:
The public face of the engine: a thin handle holding one `Rc<RefCell<ProcessEngine>>` and forwarding every call. The tab hands out clones via `get_delegate()`.
_Avoid_: "the delegate owns state" (all state and caches live in the engine)

**ViewState**:
Everything that determines how the current snapshot is displayed: filters, search, sort (keyed on `SortColumn`), resource view mode — plus a wrapping `generation` counter bumped by every mutation via `ViewState::mutate`, the universal cache invalidator.
_Avoid_: "view settings" (settings are the persisted `Config`)

**CumulativeResources**:
A process's subtree-aggregated resource totals (cpu, memory, vram, disk read/write), computed once per snapshot and cached.
_Avoid_: "cum map", "aggregate"

**Cumulative view mode**:
The display setting that shows `CumulativeResources` instead of a process's own usage. Distinct from the `CumulativeResources` data itself.

### GPU

**GpuBackend**:
Detected GPU vendor: `None`, `Nvidia`, `Amd`. Detected once at bootstrap.
**GpuDevice**:
Per-GPU telemetry: utilization, temperature, VRAM, power, clocks, fan.
**VramUsage**:
A process's VRAM in bytes, split per vendor (`nvidia`/`amd`); the split is what enables vendor filters.

### Services

**InitSystem**:
The detected init/service manager: `Systemd`, `OpenRc`, `Runit`, `Dinit`, `SysV`, or `Unknown`. Detected once at bootstrap via sentinel files and `/proc/1/exe`.
_Avoid_: "service manager" (that's the module name, not the concept)

### Config

**Config**:
The persisted settings, loaded/saved as RON at `gpuitop/config.ron`: interface, processes behavior/columns/sort, grouping, device and interface allow-lists, window size.
_Avoid_: "preferences" (the Settings tab is the UI over `Config`)

**ConfigStore**:
The single shared home for the runtime `Config` — `Rc<RefCell<Config>>` plus a generation counter. Interface: `get()` (read), `mutate(f)` (apply, bump generation, persist to disk), `generation()`. `mutate` is the only write path; the App, Settings tab, and Processes tab hold clones of the same store, so there is no hand-synced config anywhere.
_Avoid_: "config singleton", "the config" (a bare `Config` is a value; the store is the shared handle)

**ProcessSeeds**:
The two config fields that re-apply to the running processes tab on change: `default_sort` and `default_view_mode`. `ProcessSeeds::update` diffs them against the last-seen values; the tab re-seeds its `ViewState` only when one actually changed — a column toggle can't flip the live tree/list choice.
_Avoid_: "live settings" (columns/theme/refresh also apply live, but by direct read, not by seeding)

**deep_merge**:
Recursive RON layering of an overlay onto a base config — the mechanism behind CLI `--override`. Operates on `ron2::Value` because `ron::Value` loses config shape.

### Search

**fuzzy match**:
Whether a search needle fuzzy-matches a target (nucleo).
**best_fuzzy_score**:
Best fuzzy score of the search across a process's name, command, and `electron_app_name`; drives relevance ordering while a search is active.

## Relationships

- **SystemSnapshot ⊃ ProcessSnapshot**: one snapshot is the whole machine at one tick.
- **ViewState drives the pipeline**: `search` + `filters` transform `SystemSnapshot.processes` into the `match set` inside `ProcessEngine`; any `ViewState::mutate` bumps `generation`, invalidating cached rows. The Pid filter contributes scope, not matching.
- **Cumulative is derived**: `CumulativeResources` is summed bottom-up once per snapshot (lazily, whenever resource view mode is Cumulative) and consumed by cumulative sorting and the VRAM filters.
- **Graph answers every walk**: `is_descendant_of`, subtree counts, ancestor chains, and display-set computation all live in `ProcessGraph`, built once per snapshot.
- **Classification → filters**: `Filter` buckets map onto `ProcessSnapshot` flags; `Services` needs `InitSystem`; `Nvidia`/`Amd` need the per-vendor `VramUsage` split.
- **Config ↔ ViewState**: persisted defaults translate into `ViewState` at tab construction; the two `ProcessSeeds` re-apply live when changed.
- **ConfigStore ⊃ Config**: one shared handle, `mutate` the single write path.
