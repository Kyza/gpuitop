#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::{build_vram_usage, collect_gpu_info, detect_gpu};
#[cfg(target_os = "macos")]
pub use macos::{build_vram_usage, collect_gpu_info, detect_gpu};
#[cfg(target_os = "windows")]
pub use windows::{build_vram_usage, collect_gpu_info, detect_gpu};
