use std::collections::HashMap;

use gpuitop_core::model::GpuBackend;

pub fn build_vram_map(_gpu_backend: GpuBackend) -> HashMap<i32, u64> {
	HashMap::new()
}

pub fn detect_gpu() -> GpuBackend {
	GpuBackend::None
}
