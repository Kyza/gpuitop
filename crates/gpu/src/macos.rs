use std::collections::HashMap;

use gpuitop_core::model::{GpuBackend, GpuDevice, VramUsage};

pub fn build_vram_usage(_backends: &[GpuBackend]) -> HashMap<i32, VramUsage> {
	HashMap::new()
}

pub fn detect_gpu() -> Vec<GpuBackend> {
	Vec::new()
}

pub fn collect_gpu_info(_backends: &[GpuBackend]) -> Vec<GpuDevice> {
	Vec::new()
}
