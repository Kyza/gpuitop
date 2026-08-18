use std::collections::HashMap;

use gpuitop_core::model::{GpuBackend, VramUsage};

pub fn build_vram_usage(_backends: &[GpuBackend]) -> HashMap<i32, VramUsage> {
	HashMap::new()
}

pub fn detect_gpu() -> Vec<GpuBackend> {
	Vec::new()
}
