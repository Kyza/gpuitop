# 06 — Windows GPU telemetry

**What to build:** the GPU tab shows real devices and per-process VRAM on Windows. The gpu platform path currently returns empty data. Implement it via NVML (`nvml-wrapper` is already a cross-platform dependency; NVML ships a Windows DLL), producing the existing `GpuBackend`/`GpuDevice`/`VramUsage` types and feeding per-PID VRAM into the collector.

**Blocked by:** 01, 02.

**Status:** ready-for-agent

- [ ] GPU tab lists real GPU device(s) with utilization/VRAM on Windows
- [ ] Per-process VRAM shows up in the Processes tab where available
- [ ] No crash when no NVIDIA GPU present (graceful empty state)

References: `.scratch/windows-dev-env/research.md` §1.1 (gpu stub), §1.3 (nvml cross-platform note).
