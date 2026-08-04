pub mod collector;
pub mod processes;
pub mod state;

pub use state::CollectorState;

use crate::data::model::SystemSnapshot;
use std::time::Instant;

impl SystemSnapshot {
	pub fn new(state: &mut CollectorState) -> Self {
		let now = Instant::now();
		let cpu = collector::collect_cpu(state);
		let memory = collector::collect_memory();
		let total_mem = memory.total.max(1);
		let disks = collector::collect_disks(state, now);
		let networks = collector::collect_networks(state, now);
		let processes = processes::collect_processes(state, total_mem, now);

		state.prev_time = Some(now);

		SystemSnapshot {
			processes,
			cpu,
			memory,
			disks,
			networks,
			timestamp: now,
			gpu_backend: state.gpu_backend,
		}
	}
}

pub fn pid_alive(pid: i32) -> bool {
	procfs::process::Process::new(pid).is_ok()
}
