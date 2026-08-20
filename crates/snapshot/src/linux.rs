#[path = "linux/collector.rs"]
mod collector;
#[path = "linux/proc_basics.rs"]
mod proc_basics;
#[path = "linux/processes.rs"]
mod processes;

pub use proc_basics::{username_for_uid, ProcBasics};

use std::sync::atomic::Ordering;
use std::time::Instant;

use gpuitop_core::model::SystemSnapshot;

use super::CollectorState;

#[hotpath::measure]
pub fn collect_snapshot(state: &mut CollectorState) -> SystemSnapshot {
	let now = Instant::now();
	let cpu = state.collect_cpu();
	let memory = collector::collect_memory();
	let total_mem = memory.total.max(1);
	let disks = state.collect_disks(now);
	let networks = state.collect_networks(now);
	let (gpu_devices, gpu_polling_enabled) = state.collect_gpu();
	let processes = state.collect_processes(total_mem, now);

	// Async icon warmup: feed the icon names we just collected into the
	// linicon-backed path cache. We run on the background collector thread, so
	// the render thread's resolve_icon_path never blocks on the (ms-scale)
	// theme lookup — icons pop in over a few ticks.
	gpuitop_icons::warm_icon_paths(&icon_names(&processes));

	state.prev_time = Some(now);

	SystemSnapshot {
		processes,
		cpu,
		memory,
		disks,
		networks,
		timestamp: now,
		gpu_backends: state.gpu_backends.clone(),
		gpu_devices,
		gpu_polling_enabled,
	}
}

fn icon_names(
	processes: &[gpuitop_core::model::ProcessSnapshot],
) -> Vec<String> {
	let mut seen = std::collections::HashSet::new();
	processes
		.iter()
		.filter_map(|p| p.icon_name.as_deref())
		.filter(|n| seen.insert(n.to_string()))
		.map(|n| n.to_string())
		.collect()
}

impl CollectorState {
	// One per-tick GPU pass: read devices and make the backends current,
	// re-probing the system only while none are detected or on an explicit
	// re-detect request. Skipped entirely when GPU data is off.
	#[hotpath::measure]
	fn collect_gpu(&mut self) -> (Vec<gpuitop_core::model::GpuDevice>, bool) {
		if !self.gpu_data_enabled.load(Ordering::SeqCst) {
			return (Vec::new(), false);
		}
		if self.gpu_backends.is_empty()
			|| self.redetect.swap(false, Ordering::SeqCst)
		{
			self.gpu_backends = gpuitop_gpu::detect_gpu();
		}
		let devices = gpuitop_gpu::collect_gpu_info(&self.gpu_backends);
		(devices, true)
	}
}
