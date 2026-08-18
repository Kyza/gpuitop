#[path = "linux/collector.rs"]
mod collector;
#[path = "linux/processes.rs"]
mod processes;

use std::time::Instant;

use gpuitop_core::model::SystemSnapshot;

use super::CollectorState;

#[hotpath::measure]
pub fn collect_snapshot(state: &mut CollectorState) -> SystemSnapshot {
	let now = Instant::now();
	let cpu = collector::collect_cpu(state);
	let memory = collector::collect_memory();
	let total_mem = memory.total.max(1);
	let disks = collector::collect_disks(state, now);
	let networks = collector::collect_networks(state, now);
	let processes = processes::collect_processes(state, total_mem, now);
	let gpu_devices = gpuitop_gpu::collect_gpu_info(&state.gpu_backends);

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
	}
}

pub fn pid_alive(pid: i32) -> bool {
	procfs::process::Process::new(pid).is_ok()
}
