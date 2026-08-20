# 03 — Honor the config at the collector seam

Status: resolved
Type: task
Blocked by: —

## Goal

`VramPolling` actually gates the per-tick VRAM poll; the collector's state
seam stops being a bare field list.

## Friction

- `VramPolling` (Auto/On/Off) is persisted (`core/src/config.rs:99`) and
  editable (`settings/src/processes.rs`), but never consulted at collection
  time — the collector shells out to NVML + `rocm-smi` every tick
  unconditionally (`snapshot/src/linux/processes.rs:25`), plus a second
  per-tick GPU read for device telemetry (`linux.rs:21`).
- `CollectorState` (`snapshot/src/lib.rs:36-77`) is a 10-field pub struct
  with no pub methods; the pub fields are the contract between lib.rs and the
  linux/ submodules (the cfg-dispatch seam, lib.rs:79-91).
- Dead surface: `core_history: HashMap<usize, Vec<f32>>` is written every
  tick (`collector.rs:73-77`) and never read anywhere; `pid_alive` is
  exported (`lib.rs:87-91`) and never called.
- Sub-collector internals untestable in isolation: `collect_processes`
  (~10-212) mixes /proc reads, delta math, the GUI-starttime cache
  (~68-78), icon lookup (~102-105), electron detection (~192), and three
  intermediate map builds. The delta math has no unit tests; one test
  (`test_gui_detection`) asserts `gui_count > 0` and fails headless.

## Solution sketch

- Thread the polling setting into `CollectorState` at construction; gate the
  per-tick VRAM read.
- Give the state methods instead of a contract-by-field-list.
- Delete `core_history` and `pid_alive` (deletion test: nothing concentrates).

## Wins

- config interface stops advertising behavior that doesn't run.
- deep interface (`collect_snapshot`) stays; its seam stops being the field
  list.

## Acceptance

- [x] `VramPolling::Off` produces zero NVML/rocm-smi calls per tick.
- [x] `core_history` and `pid_alive` deleted with no behavior change.
- [x] Collector delta math has unit tests (pure extraction, live system
      not required).

## Answer

Implemented. Design settled by grilling (rounds 1–4):

- **`VramPolling` → `GpuData` (binary On/Off, default On)**, covering all GPU
  reads, not just VRAM. Config field + RON key renamed `vram_polling` →
  `gpu_data` (old key silently ignored on upgrade → defaults to On). Settings
  label "GPU data".
- **Live threading via mirrors**: `Arc<AtomicBool>` `gpu_data` (enable) +
  `redetect` (one-shot) shared with the collector thread — the same idiom as
  `refresh_ms`. `set_gpu_data` writes config + mirror; the Settings page's
  Re-detect button sets the flag (disabled while Off).
- **Gate both reads**: when Off the collector skips `build_vram_usage`
  (per-PID VRAM) *and* `collect_gpu_info` (per-device telemetry) — zero
  NVML/rocm-smi calls. When On, backends are re-probed only while none are
  detected or on an explicit re-detect (hotplug appears live; `snapshot`
  carries the fresh backends).
- **CollectorState encapsulated**: the 10 pub fields went private; the
  sub-collectors (`collect_cpu`, `collect_disks`, `collect_networks`,
  `collect_processes`) are now `impl CollectorState` methods in the linux
  modules; `collect_snapshot` is a thin driver. Adds `collect_gpu`.
- **UI reflects the state**: `SystemSnapshot.gpu_polling_enabled` stamps the
  per-tick decision — GPU tab shows "GPU data is off." vs "No GPU detected."
  vs live devices; status bar "GPU:" shows Off when disabled.
- **Deletions**: `core_history` (write-only; the perf-tab history is
  UI-side) and `pid_alive` (never called) removed, incl. the macos/windows
  stubs.
- **Delta math extracted**: `usage_pct` (core busy ratio) and
  `proc_cpu_pct` (per-process %) are pure fns with unit tests — no live
  system needed. Also dropped the `bench_collector_tick` test (benchmarks are
  hotpath's job). 278 passing.
- Docs updated: core/snapshot/settings CONTEXT.md.
