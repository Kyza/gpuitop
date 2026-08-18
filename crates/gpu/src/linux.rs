use std::collections::HashMap;

use gpuitop_core::model::{GpuBackend, VramUsage};

#[hotpath::measure]
fn build_nvidia_vram_map() -> HashMap<i32, u64> {
	let nvml = match nvml_wrapper::Nvml::init() {
		Ok(n) => n,
		Err(_) => return HashMap::new(),
	};
	let count = match nvml.device_count() {
		Ok(c) => c,
		Err(_) => return HashMap::new(),
	};
	let mut map = HashMap::new();
	for i in 0..count {
		if let Ok(device) = nvml.device_by_index(i) {
			for proc_list in [
				device.running_compute_processes(),
				device.running_graphics_processes(),
			] {
				if let Ok(procs) = proc_list {
					for p in &procs {
						if let nvml_wrapper::enums::device::UsedGpuMemory::Used(mem) = p.used_gpu_memory {
							*map.entry(p.pid as i32).or_insert(0) += mem;
						}
					}
				}
			}
		}
	}
	map
}

#[hotpath::measure]
fn build_rocm_vram_map() -> HashMap<i32, u64> {
	let output = match std::process::Command::new("rocm-smi")
		.args(["--showpids", "--csv"])
		.output()
	{
		Ok(o) => o,
		Err(_) => return HashMap::new(),
	};
	if !output.status.success() {
		return HashMap::new();
	}
	let stdout = String::from_utf8_lossy(&output.stdout);
	let mut map = HashMap::new();
	for line in stdout.lines() {
		let mut parts = line.split(',');
		let pid: i32 = match parts.next().and_then(|s| s.trim().parse().ok())
		{
			Some(p) => p,
			None => continue,
		};
		for part in parts {
			let trimmed = part.trim();
			if let Some(s) = trimmed.strip_suffix(" MB") {
				if let Ok(mb) = s.parse::<u64>() {
					*map.entry(pid).or_insert(0) += mb * 1024 * 1024;
				}
			}
		}
	}
	map
}

#[hotpath::measure]
pub fn build_vram_usage(backends: &[GpuBackend]) -> HashMap<i32, VramUsage> {
	let mut map: HashMap<i32, VramUsage> = HashMap::new();
	for backend in backends {
		let per_vendor = match backend {
			GpuBackend::Nvidia => build_nvidia_vram_map(),
			GpuBackend::Amd => build_rocm_vram_map(),
			GpuBackend::None => continue,
		};
		for (pid, bytes) in per_vendor {
			let entry = map.entry(pid).or_default();
			match backend {
				GpuBackend::Nvidia => entry.nvidia += bytes,
				GpuBackend::Amd => entry.amd += bytes,
				GpuBackend::None => {}
			}
		}
	}
	map
}

pub fn detect_gpu() -> Vec<GpuBackend> {
	let mut backends = Vec::new();
	if nvml_wrapper::Nvml::init().is_ok() {
		backends.push(GpuBackend::Nvidia);
	}
	if std::process::Command::new("rocm-smi")
		.arg("--showpids")
		.output()
		.map(|o| o.status.success())
		.unwrap_or(false)
	{
		backends.push(GpuBackend::Amd);
	}
	backends
}
