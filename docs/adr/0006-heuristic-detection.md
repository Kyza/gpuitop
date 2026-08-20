# Dependency-free heuristic platform detection

All runtime classification is heuristic, never a DB or daemon query: GPU backends are probed via NVML init / `rocm-smi`, init systems via sentinel files + `/proc/1/exe`, services via cgroup suffixes and process-name parentage, Electron apps via `--type=` argv chains, icons via a minimal `.desktop` parser, and GUI-ness via `DISPLAY`/`WAYLAND_DISPLAY` in the process environment.

This keeps the app dependency-light and portable at the cost of classification accuracy — the `System`/`Services` filter buckets are precise but narrower than their tooltips, and icon/app-name resolution falls back to heuristics. Each heuristic is pinned by unit tests where it matters.
