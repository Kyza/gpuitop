# Per-OS `#[cfg(target_os)]` free-function dispatch

Every platform feature (gpu, icons, snapshot, window_picker, elevation, service_manager, properties) follows one shape: the crate root declares `#[cfg(target_os)] mod linux/macos/windows;` and `pub use`s matching free functions from the platform module. Shared types live in the crate root (`lib.rs`); platform behavior lives in `linux.rs`/`macos.rs`/`windows.rs`. Directory-based platforms (snapshot, window_picker) use a `linux.rs` entry that declares `#[path = "linux/*.rs"]` submodules.

This keeps one compile story per OS with no runtime dispatch, at the cost of triple-implementing each interface where platforms genuinely differ. Constructors/associated fns become free fns (`collect_snapshot`, `load_cache`) so the platform modules stay plain `pub fn` collections.
