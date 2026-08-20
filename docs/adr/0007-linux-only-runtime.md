# Linux-only runtime features; non-Linux compile-time stubs

GPU telemetry, icon resolution, window picking, elevation, process properties, and process collection are Linux-only. macOS and Windows modules are compile-complete stubs: empty collections, `false` liveness, `None` properties, explicit "not supported" errors. The `#[cfg(target_os)]` dispatch keeps the whole app buildable everywhere while runtime behavior on non-Linux is degraded rather than broken.

This is a deliberate carve-out, not an unfinished port: a future macOS/Windows port would implement per-platform modules (NVML equivalents, `.app` bundle icons, etc.) behind the same interface.
