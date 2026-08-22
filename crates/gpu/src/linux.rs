use std::collections::HashMap;
use std::sync::Mutex;

use gpuitop_core::model::{GpuBackend, GpuDevice, VramUsage};
use nvml_wrapper::enum_wrappers::device::{Clock, TemperatureSensor};
use nvml_wrapper::Nvml;

// NVML is process-global. nvmlInit opens a driver fd that nvmlShutdown does
// not fully close, so a fresh Nvml::init() per call leaks one fd (measured:
// exactly 1 fd per init/drop). Cache a single handle for the process and
// reuse it; the fd count then stays flat instead of creeping every tick.
static NVML: Mutex<Option<Nvml>> = Mutex::new(None);

fn with_nvml<R>(f: impl FnOnce(&Nvml) -> R) -> Option<R> {
	let mut guard = NVML.lock().unwrap();
	if guard.is_none() {
		*guard = Nvml::init().ok();
	}
	guard.as_ref().map(f)
}

#[hotpath::measure]
fn build_nvidia_vram_map() -> HashMap<i32, u64> {
	with_nvml(|nvml| {
		let mut map = HashMap::new();
		let count = match nvml.device_count() {
			Ok(c) => c,
			Err(_) => return map,
		};
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
	})
	.unwrap_or_default()
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
	if with_nvml(|_| ()).is_some() {
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

#[hotpath::measure]
fn collect_nvidia_devices() -> Vec<GpuDevice> {
	with_nvml(|nvml| {
		let mut devices = Vec::new();
		let count = match nvml.device_count() {
			Ok(c) => c,
			Err(_) => return devices,
		};
		for i in 0..count {
			let Ok(device) = nvml.device_by_index(i) else {
				continue;
			};
			let name = device.name().unwrap_or_default();
			let utilization =
				device.utilization_rates().map(|u| u.gpu).unwrap_or(0);
			let temperature =
				device.temperature(TemperatureSensor::Gpu).unwrap_or(0);
			let (vram_total, vram_used) = match device.memory_info() {
				Ok(info) => (info.total, info.used),
				Err(_) => (0, 0),
			};
			let power_watts = device
				.power_usage()
				.map(|m| m as f32 / 1000.0)
				.unwrap_or(0.0);
			let core_clock_mhz =
				device.clock_info(Clock::Graphics).unwrap_or(0);
			let memory_clock_mhz =
				device.clock_info(Clock::Memory).unwrap_or(0);
			let fan_percent = device.fan_speed(0).unwrap_or(0);

			devices.push(GpuDevice {
				backend: GpuBackend::Nvidia,
				name,
				utilization,
				temperature,
				vram_total,
				vram_used,
				power_watts,
				core_clock_mhz,
				memory_clock_mhz,
				fan_percent,
			});
		}
		devices
	})
	.unwrap_or_default()
}

#[hotpath::measure]
fn collect_rocm_devices() -> Vec<GpuDevice> {
	let output = match std::process::Command::new("rocm-smi")
		.args([
			"--showproductname",
			"--showtemp",
			"--showuse",
			"--showmeminfo",
			"vram",
			"--csv",
		])
		.output()
	{
		Ok(o) => o,
		Err(_) => return Vec::new(),
	};
	if !output.status.success() {
		return Vec::new();
	}
	let stdout = String::from_utf8_lossy(&output.stdout);
	let mut lines = stdout.lines();
	let Some(header) = lines.next() else {
		return Vec::new();
	};
	let cols: Vec<String> =
		header.split(',').map(|s| s.trim().to_lowercase()).collect();
	let find = |needle: &str| cols.iter().position(|c| c.contains(needle));
	let name_idx = find("series").or_else(|| find("model"));
	let temp_idx = find("temperature");
	let util_idx = find("gpu use");
	let total_idx = find("total memory");
	let used_idx = find("used memory");

	let mut devices = Vec::new();
	for line in lines {
		let parts: Vec<&str> = line.split(',').map(str::trim).collect();
		if parts.is_empty() || parts.iter().all(|p| p.is_empty()) {
			continue;
		}
		let get_u64 = |idx: Option<usize>| {
			idx.and_then(|i| parts.get(i))
				.and_then(|v| v.parse::<u64>().ok())
				.unwrap_or(0)
		};
		let get_u32 = |idx: Option<usize>| {
			idx.and_then(|i| parts.get(i))
				.and_then(|v| v.parse::<u32>().ok())
				.unwrap_or(0)
		};
		devices.push(GpuDevice {
			backend: GpuBackend::Amd,
			name: name_idx
				.and_then(|i| parts.get(i))
				.unwrap_or(&"")
				.to_string(),
			utilization: get_u32(util_idx),
			temperature: get_u32(temp_idx),
			vram_total: get_u64(total_idx),
			vram_used: get_u64(used_idx),
			power_watts: 0.0,
			core_clock_mhz: 0,
			memory_clock_mhz: 0,
			fan_percent: 0,
		});
	}
	devices
}

#[hotpath::measure]
pub fn collect_gpu_info(backends: &[GpuBackend]) -> Vec<GpuDevice> {
	let mut devices = Vec::new();
	for backend in backends {
		match backend {
			GpuBackend::Nvidia => devices.extend(collect_nvidia_devices()),
			GpuBackend::Amd => devices.extend(collect_rocm_devices()),
			GpuBackend::None => {}
		}
	}
	devices
}

#[cfg(test)]
mod tests {
	use super::*;

	fn fd_count() -> usize {
		std::fs::read_dir("/proc/self/fd")
			.map(|d| d.count())
			.unwrap_or(0)
	}

	// Regression: reusing the cached NVML handle must not open new fds.
	// Before the once-init fix, each raw Nvml::init() leaked exactly one fd
	// (nvmlInit opens a driver fd that nvmlShutdown doesn't fully close); the
	// two per-tick init calls in the GPU path were the 2 fds/tick leak that
	// exhausted the fd limit and blanked the process list after ~8 minutes.
	#[test]
	fn cached_nvml_handle_does_not_leak_fds() {
		for _ in 0..20 {
			let _ = with_nvml(|_| ());
		}
		let before = fd_count();
		for _ in 0..200 {
			let _ = with_nvml(|_| ());
		}
		let after = fd_count();
		assert!(
			after as isize - before as isize <= 8,
			"cached nvml handle leaks fds: {before} -> {after}"
		);
	}
}
