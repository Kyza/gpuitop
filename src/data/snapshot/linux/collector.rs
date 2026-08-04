use super::state::*;
use crate::data::model::*;
use procfs::{Current, CurrentSI};
use std::collections::HashMap;
use std::time::Instant;

const HISTORY_LEN: usize = 60;

#[hotpath::measure]
pub fn collect_cpu(state: &mut CollectorState) -> CpuInfo {
	let Ok(kernel) = procfs::KernelStats::current() else {
		return CpuInfo {
			cores: Vec::new(),
			overall_percent: 0.0,
		};
	};

	let mut cur_totals: Vec<(u64, u64)> = Vec::new();
	let times = std::iter::once(&kernel.total).chain(kernel.cpu_time.iter());
	for time in times {
		let total =
			time.user
				+ time.nice + time.system
				+ time.idle + time.iowait.unwrap_or(0)
				+ time.irq.unwrap_or(0)
				+ time.softirq.unwrap_or(0)
				+ time.steal.unwrap_or(0)
				+ time.guest.unwrap_or(0)
				+ time.guest_nice.unwrap_or(0);
		let idle = time.idle + time.iowait.unwrap_or(0);
		cur_totals.push((total, idle));
	}

	let mut cores = Vec::new();

	if let Some(ref prev) = state.prev_cpu {
		for (i, (total, idle)) in cur_totals.iter().enumerate() {
			if i >= prev.totals.len() {
				continue;
			}
			let (prev_total, prev_idle) = prev.totals[i];
			let dt = total.saturating_sub(prev_total) as f32;
			let di = idle.saturating_sub(prev_idle) as f32;
			let usage = if dt > 0.0 {
				((dt - di) / dt * 100.0).clamp(0.0, 100.0)
			} else {
				0.0
			};

			if i == 0 {
				cores.push(CpuCore {
					index: 0,
					usage_percent: usage,
				});
			} else {
				let cidx = i - 1;
				let history = state.core_history.entry(cidx).or_default();
				history.push(usage);
				if history.len() > HISTORY_LEN {
					history.remove(0);
				}
				cores.push(CpuCore {
					index: cidx,
					usage_percent: usage,
				});
			}
		}
	}

	if cores.is_empty() {
		let ncores = cur_totals.len().saturating_sub(1).max(1);
		for i in 0..ncores {
			cores.push(CpuCore {
				index: i,
				usage_percent: 0.0,
			});
		}
	}

	let overall = cores.first().map(|c| c.usage_percent).unwrap_or(0.0);

	state.prev_cpu = Some(PrevCpu { totals: cur_totals });

	CpuInfo {
		cores,
		overall_percent: overall,
	}
}

#[hotpath::measure]
pub fn collect_memory() -> MemoryInfo {
	let Ok(mem) = procfs::Meminfo::current() else {
		return MemoryInfo {
			total: 0,
			used: 0,
			available: 0,
			cached: 0,
			swap_used: 0,
		};
	};

	let total = mem.mem_total;
	let free = mem.mem_free;
	let buffers = mem.buffers;
	let cached = mem.cached + mem.s_reclaimable.unwrap_or(0);
	let available = mem.mem_available.unwrap_or(0);
	let used = total.saturating_sub(free + buffers + cached);
	let swap_used = mem.swap_total.saturating_sub(mem.swap_free);

	MemoryInfo {
		total,
		used,
		available,
		cached,
		swap_used,
	}
}

#[hotpath::measure]
pub fn collect_disks(
	state: &mut CollectorState,
	now: Instant,
) -> Vec<DiskInfo> {
	let Ok(disks) = procfs::diskstats() else {
		return Vec::new();
	};
	let mut cur: HashMap<String, (u64, u64)> = HashMap::new();
	for d in &disks {
		cur.insert(
			d.name.clone(),
			(d.sectors_read * 512, d.sectors_written * 512),
		);
	}

	let mut result = Vec::new();
	if let Some(ref prev_time) = state.prev_time {
		let elapsed = now.duration_since(*prev_time).as_secs_f64().max(0.001);
		for (name, (cr, cw)) in &cur {
			if let Some(prev) = state.prev_disk.get(name) {
				result.push(DiskInfo {
					device: name.clone(),
					read_bytes_per_sec: cr.saturating_sub(prev.read_bytes)
						as f64 / elapsed,
					write_bytes_per_sec: cw.saturating_sub(prev.write_bytes)
						as f64 / elapsed,
				});
			}
		}
	}
	state.prev_disk = cur
		.into_iter()
		.map(|(k, (r, w))| {
			(
				k,
				PrevDisk {
					read_bytes: r,
					write_bytes: w,
				},
			)
		})
		.collect();
	result
}

#[hotpath::measure]
pub fn collect_networks(
	state: &mut CollectorState,
	now: Instant,
) -> Vec<NetInfo> {
	let Ok(nets) = procfs::net::dev_status() else {
		return Vec::new();
	};
	let mut cur: HashMap<String, (u64, u64)> = HashMap::new();
	for (name, dev) in nets {
		cur.insert(name, (dev.recv_bytes, dev.sent_bytes));
	}

	let mut result = Vec::new();
	if let Some(ref prev_time) = state.prev_time {
		let elapsed = now.duration_since(*prev_time).as_secs_f64().max(0.001);
		for (name, (crx, ctx)) in &cur {
			if let Some(prev) = state.prev_net.get(name) {
				result.push(NetInfo {
					interface: name.clone(),
					rx_bytes_per_sec: crx.saturating_sub(prev.rx_bytes)
						as f64 / elapsed,
					tx_bytes_per_sec: ctx.saturating_sub(prev.tx_bytes)
						as f64 / elapsed,
				});
			}
		}
	}
	state.prev_net = cur
		.into_iter()
		.map(|(k, (r, t))| {
			(
				k,
				PrevNet {
					rx_bytes: r,
					tx_bytes: t,
				},
			)
		})
		.collect();
	result
}
