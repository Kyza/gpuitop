# Performance

The Performance tab: rolling-history charts plus per-resource sub-tabs (CPU, Memory, GPU, Disks, Network). Rendering never reads /proc — it consumes the shared `SystemSnapshot` and derives a lightweight sample per tick.

## Language

**PerformanceTab**:
The tab entity: an `Rc<SystemSnapshot>`, the active sub-tab index, and a rolling `History`.
**Sub-tab**:
One of CPU, Memory, GPU, Disks, Network — the performance tab's own internal pages, distinct from the app's main tabs.
_Avoid_: "tab" alone (ambiguous with main tabs)

**Sample**:
One tick's derived metrics: cpu%, mem used, network rx/tx, disk read/write, GPU utilization — the compact input to the charts.
**History**:
A ring buffer of the last 60 `Sample`s; chart x-axis labels are monotonically increasing tick numbers.
_Avoid_: "time series" (there are no timestamps — labels are tick counters)

**Sub-tab cards**:
Per-resource widgets: CPU overall ring + per-core tiles, memory pie, per-GPU device cards (VRAM ring, utilization/temp/power/clocks/fan), disk/network rates + area charts.
**Top VRAM consumers**:
The top-10 processes by `vram.total()` listed on the GPU sub-tab.

**Widgets**:
Shared sub-tab building blocks: `card`, `stat`, `stat_tiles`, `chart_box`, `format_rate` ("X B/s").
**Load color**:
The >80 danger / >50 warning threshold coloring for CPU/memory.

## Relationships

- **Performance → Core**: consumes `CpuInfo`/`MemoryInfo`/`GpuDevice`/`SystemSnapshot` directly.
- **Sample vs Snapshot**: `Sample` is derived from a snapshot tick; the snapshot is never re-read for chart data.
- **GPU → Performance**: `GpuDevice` renders the device cards; `GpuBackend` maps onto tag colors.
- **Multi-GPU**: GPU utilization is the max across devices — a deliberate aggregation.
