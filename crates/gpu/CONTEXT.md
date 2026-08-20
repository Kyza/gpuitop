# GPU

Detects GPU backends once at bootstrap and collects GPU telemetry: per-device stats for the Performance tab and per-PID VRAM for the process table. Pure data — no gpui.

## Language

**GpuBackend**:
Detected GPU vendor: `None`, `Nvidia`, `Amd` — probed by NVML init succeeding (Nvidia) or `rocm-smi --showpids` succeeding (AMD). Detected once at bootstrap, threaded through the app and stamped on the snapshot.
_Avoid_: "vendor" alone (that's a UI display concern)

**GpuDevice**:
Per-GPU telemetry: utilization, temperature, VRAM total/used, power, core/memory clocks, fan.
**VramUsage**:
A process's VRAM in bytes, split per vendor (`nvidia`/`amd`); the split enables the vendor filters.
_Avoid_: "VRAM" alone when referring to the split data (plain VRAM is a byte count)

**VRAM attribution**:
Merging per-vendor process-VRAM maps into one PID-keyed `VramUsage`, stamped onto each `ProcessSnapshot.vram` each tick.
**Per-device polling**:
The separate NVML/rocm-smi read for per-device stats — deliberately independent of the hot per-tick VRAM attribution path.

## Relationships

- **GPU → Core**: `GpuBackend`/`GpuDevice`/`VramUsage` are core types; this crate produces them.
- **GPU → Snapshot**: per-PID VRAM feeds `ProcessSnapshot.vram` each tick.
- **GPU → Performance**: `GpuDevice` renders the GPU sub-tab cards.
