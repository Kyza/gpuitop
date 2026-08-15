use std::collections::HashMap;

use gpuitop_core::model::GpuBackend;

use super::{GpuInterface, Platform};

impl_interface! {
	fn build_vram_map(_gpu_backend: GpuBackend) -> HashMap<i32, u64> {
		HashMap::new()
	}

	fn detect_gpu() -> GpuBackend {
		GpuBackend::None
	}
}
