use crate::data::model::GpuBackend;
use std::collections::HashMap;

pub fn detect_gpu() -> GpuBackend {
	GpuBackend::None
}

pub fn build_vram_map(_gpu_backend: GpuBackend) -> HashMap<i32, u64> {
	HashMap::new()
}
