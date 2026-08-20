# Context Map

## Contexts

- [Core](./crates/core/CONTEXT.md): the data model — snapshots, filters, sorts, view state, config. Zero UI.
- [Snapshot](./crates/snapshot/CONTEXT.md): collects one `SystemSnapshot` per tick on a background thread.
- [Processes](./crates/processes/CONTEXT.md): the process table/tree view — filtering, sorting, pinning, signals.
- [GPU](./crates/gpu/CONTEXT.md): detects GPU backends and collects per-device + per-PID VRAM telemetry.
- [Icons](./crates/icons/CONTEXT.md): resolves process icons from Linux `.desktop` files.
- [Window Picker](./crates/window_picker/CONTEXT.md): "click a window to find its process" crosshair tool.
- [Elevation](./crates/elevation/CONTEXT.md): relaunches the app as root via `pkexec`.
- [Components](./crates/components/CONTEXT.md): shared UI building blocks — selectable text, tags, themes, icons.
- [Properties](./crates/properties/CONTEXT.md): per-process detail window (data collection + popup UI).
- [Performance](./crates/performance/CONTEXT.md): rolling-history charts (CPU, memory, GPU, disks, network).
- [Settings](./crates/settings/CONTEXT.md): the Settings tab and its config-mutation gateway.
- [gpuitop](./crates/gpuitop/CONTEXT.md): the app shell — window, tabs, CLI, collector thread, elevation state.

## Relationships

- **Snapshot → Core**: `collect_snapshot(&mut CollectorState)` produces the `SystemSnapshot` data model.
- **GPU → Core**: produces `GpuBackend`/`GpuDevice`/`VramUsage`; the snapshot collector consumes per-PID VRAM.
- **Icons → Snapshot**: the `Arc<DesktopEntryCache>` is shared into `CollectorState`; the collector stamps each process with `icon_name`.
- **Window Picker → Processes**: `PickedWindow` becomes a search keyword that pre-fills the process filter.
- **Elevation → gpuitop**: the titlebar shield button calls `relaunch_elevated()` and quits.
- **Processes → Core/Components**: reads `SystemSnapshot` + `ViewState`; renders via components' tags/icons; opens a `PropertiesWindow`.
- **Components → Core**: tags and filter icons map onto core `Filter`/`SortColumn` concepts.
- **Performance → Core**: consumes `CpuInfo`/`MemoryInfo`/`GpuDevice`/`SystemSnapshot`.
- **Settings → Core/Components**: maps config fields to UI pages; reuses the theme menu.
- **Properties → Core**: `ProcessProperties` is a richer per-process type alongside `ProcessSnapshot`.
- **gpuitop → everything**: hosts the `App` shell, owns the collector thread, and wires all tabs together.
