use std::collections::HashMap;

use gpuitop_core::model::GpuBackend;
use oswap::{define_interface, define_platforms};

define_interface! { Platform, GpuInterface, impl_interface,
	pub fn build_vram_map(gpu_backend: GpuBackend) -> HashMap<i32, u64>;
	pub fn detect_gpu() -> GpuBackend;
}

define_platforms![
	{ file: "linux", cfg: target_os = "linux" },
	{ file: "macos", cfg: target_os = "macos" },
	{ file: "windows", cfg: target_os = "windows" },
];
