# Snapshot

Collects one `SystemSnapshot` per tick on a background thread, holding mutable `CollectorState` between ticks. The interface is one free function: `collect_snapshot(&mut CollectorState) -> SystemSnapshot`. Linux reads /proc via the `procfs` crate; macOS/Windows are stubs.

## Language

**CollectorState**:
The mutable tick-to-tick accumulator: previous samples for CPU/processes/disks/networks, the uid→username cache, the shared icon cache, the detected GPU backends, and the two shared GPU atomics (`gpu_data` enable flag + one-shot re-detect request). Fields are private; the sub-collectors are `impl CollectorState` methods in the platform modules. Created once; mutated in place by every `collect_snapshot` call.
_Avoid_: "collector" alone (that's the thread/host, not the state)

**collect_snapshot**:
The single per-tick entry point: runs the five sub-collectors (cpu, memory, disks, networks, processes) in order, packages a `SystemSnapshot`, and stamps the previous-time sample. A thin driver — the state methods do the work.
_Avoid_: "refresh", "tick" (a tick is the interval; this is the work)

**Snapshot tick**:
One collector iteration, spaced by the refresh interval. The first tick is a throwaway warm-up: all rates are zero because deltas need two samples, so `prev_time` is stamped at the *end* of each tick.
_Avoid_: "frame" (rendering frames are unrelated)

**prev sample**:
The previous tick's raw cumulative value for a resource — CPU jiffies per core, per-device disk/network bytes, per-process CPU ticks. Rates are computed as `(now − prev) / elapsed`.
_Avoid_: "delta base", "last sample"

**GPU data gating**:
The `GpuData` setting mirrored as a shared `Arc<AtomicBool>`. When on, each tick re-probes for GPU backends only while none are detected (or on an explicit re-detect request), then reads per-device telemetry and per-PID VRAM. When off, none of the NVML/rocm-smi reads run — the snapshot's `gpu_polling_enabled` goes false so the UI can say "GPU data is off" rather than "no GPU detected".
_Avoid_: "VRAM polling" (it gates all GPU reads, not just VRAM)

**GUI detection**:
Reading `DISPLAY`/`WAYLAND_DISPLAY` from a process environment once, cached per PID, and only trusted when the PID's `starttime` still matches — a recycled PID can't inherit another process's GUI flag.
_Avoid_: "display check"

**Kernel thread skip**:
Kernel threads (ppid 2 or bracketed name) skip cmdline/cgroup/environ/VRAM reads entirely during collection — they are cheap by design.
_Avoid_: "kernel shortcut"

**Electron detection**:
A post-pass over collected processes that walks `--type=` subprocess chains up to a validated root and tags the members as an Electron app with a friendly app name derived from argv.
_Avoid_: "chrome walk"

**ProcBasics**:
The shared per-PID /proc read consumed by both the tick collector and the properties window: stat + status + cmdline read once (cmdline skipped for kernel threads), plus the shared derivations — `name()` (argv0 basename or comm), `cpu_tick_sum()`, `cpu_time_secs()`, `username_for_uid()`. `environ` stays out and is read on demand by each consumer (the tick collector's GUI cache gates it). Single-consumer fields (exe, limits, smaps, …) stay consumer-side.
_Avoid_: "process info" (ProcessProperties is the window's richer record)

**ReadError**:
Why a per-PID /proc read failed: `Dead` (process exited or never existed) vs `Unavailable` (exists but unreadable — e.g. permissions). Consumers render the two differently: dead stops polling, unavailable keeps polling with a degraded warning.
_Avoid_: conflating the two as one "not found"

**prev_proc**:
The per-PID previous-tick map, replaced wholesale each tick with the just-seen PIDs.
**user_cache**:
uid → username resolution, filled on miss only.
**desktop_cache**:
The shared `Arc<DesktopEntryCache>` (from the icons crate) used to stamp each process with an `icon_name`.

## Relationships

- **CollectorState → collect_snapshot**: the free function mutates the state and returns a fresh immutable `SystemSnapshot`; the sub-collectors are methods on the state.
- **Snapshot → Core**: the produced `SystemSnapshot` is the core data model; `ReadError`/`ProcBasics` are snapshot types consumed by Properties.
- **Icons → Snapshot**: `desktop_cache` is built once in the bin and shared into the collector thread.
- **Settings → Snapshot**: the `GpuData` enable flag and re-detect request are shared `Arc<AtomicBool>`s written by the Settings tab, read per tick by the collector.
- **Warm-up**: meaningful rates appear from tick 2 on; tick 1 is zeros.
