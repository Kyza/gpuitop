use gpuitop_core::model::*;
use procfs::{Current, CurrentSI};
use std::collections::HashMap;
use std::time::Instant;

use crate::{CollectorState, PrevCpu, PrevDisk, PrevNet};

impl CollectorState {
	#[hotpath::measure]
	pub fn collect_cpu(&mut self) -> CpuInfo {
		let Ok(kernel) = procfs::KernelStats::current() else {
			return CpuInfo {
				cores: Vec::new(),
				overall_percent: 0.0,
				model_name: String::new(),
				temperature: 0.0,
			};
		};
		let cpuinfo = procfs::CpuInfo::current().ok();
		let model_name = cpuinfo
			.as_ref()
			.and_then(|ci| ci.model_name(0))
			.unwrap_or_default()
			.to_string();
		let temperature = read_cpu_temp();
		let core_freq_mhz = |cidx: usize| -> u32 {
			cpuinfo
				.as_ref()
				.and_then(|ci| ci.get_field(cidx, "cpu MHz"))
				.and_then(|s| s.parse::<f32>().ok())
				.map(|f| f as u32)
				.unwrap_or(0)
		};

		let mut cur_totals: Vec<(u64, u64)> = Vec::new();
		let times =
			std::iter::once(&kernel.total).chain(kernel.cpu_time.iter());
		for time in times {
			let total = time.user
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
		let mut overall = 0.0f32;

		if let Some(ref prev) = self.prev_cpu {
			for (i, (total, idle)) in cur_totals.iter().enumerate() {
				if i >= prev.totals.len() {
					continue;
				}
				let (prev_total, prev_idle) = prev.totals[i];
				let usage = usage_pct(
					total.saturating_sub(prev_total) as f32,
					idle.saturating_sub(prev_idle) as f32,
				);

				if i == 0 {
					overall = usage;
				} else {
					let cidx = i - 1;
					cores.push(CpuCore {
						index: cidx,
						usage_percent: usage,
						frequency_mhz: core_freq_mhz(cidx),
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
					frequency_mhz: core_freq_mhz(i),
				});
			}
		}

		self.prev_cpu = Some(PrevCpu { totals: cur_totals });

		CpuInfo {
			cores,
			overall_percent: overall,
			model_name,
			temperature,
		}
	}

	#[hotpath::measure]
	pub fn collect_disks(&mut self, now: Instant) -> Vec<DiskInfo> {
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
		if let Some(ref prev_time) = self.prev_time {
			let elapsed =
				now.duration_since(*prev_time).as_secs_f64().max(0.001);
			for (name, (cr, cw)) in &cur {
				if let Some(prev) = self.prev_disk.get(name) {
					result.push(DiskInfo {
						device: name.clone(),
						read_bytes_per_sec: cr.saturating_sub(prev.read_bytes)
							as f64 / elapsed,
						write_bytes_per_sec: cw
							.saturating_sub(prev.write_bytes)
							as f64 / elapsed,
					});
				}
			}
		}
		self.prev_disk = cur
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
	pub fn collect_networks(&mut self, now: Instant) -> Vec<NetInfo> {
		let Ok(nets) = procfs::net::dev_status() else {
			return Vec::new();
		};
		let mut cur: HashMap<String, (u64, u64)> = HashMap::new();
		for (name, dev) in nets {
			cur.insert(name, (dev.recv_bytes, dev.sent_bytes));
		}

		let mut result = Vec::new();
		if let Some(ref prev_time) = self.prev_time {
			let elapsed =
				now.duration_since(*prev_time).as_secs_f64().max(0.001);
			for (name, (crx, ctx)) in &cur {
				if let Some(prev) = self.prev_net.get(name) {
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
		self.prev_net = cur
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
}

// CPU core busy ratio from one tick's deltas, clamped to [0, 100].
fn usage_pct(total_delta: f32, idle_delta: f32) -> f32 {
	if total_delta > 0.0 {
		((total_delta - idle_delta) / total_delta * 100.0).clamp(0.0, 100.0)
	} else {
		0.0
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

fn read_cpu_temp() -> f32 {
	let Ok(entries) = std::fs::read_dir("/sys/class/thermal") else {
		return 0.0;
	};
	let mut candidates: Vec<(String, f32)> = Vec::new();
	for entry in entries.flatten() {
		let name = entry.file_name();
		let name = name.to_string_lossy();
		if !name.starts_with("thermal_zone") {
			continue;
		}
		let base = entry.path();
		let zone_type = std::fs::read_to_string(base.join("type"))
			.unwrap_or_default()
			.trim()
			.to_lowercase();
		let Ok(temp_str) = std::fs::read_to_string(base.join("temp")) else {
			continue;
		};
		let Ok(milli) = temp_str.trim().parse::<f32>() else {
			continue;
		};
		let temp = milli / 1000.0;
		if temp > 0.0 {
			candidates.push((zone_type, temp));
		}
	}
	let is_cpu = |t: &str| {
		t.contains("cpu")
			|| t.contains("pkg")
			|| t.contains("coretemp")
			|| t.contains("k10temp")
			|| t.contains("x86")
			|| t.contains("soc")
	};
	candidates
		.iter()
		.find(|(t, _)| is_cpu(t))
		.or_else(|| candidates.first())
		.map(|(_, t)| *t)
		.unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn usage_pct_steady_state() {
		assert_eq!(usage_pct(100.0, 25.0), 75.0);
	}

	#[test]
	fn usage_pct_no_delta_is_zero() {
		assert_eq!(usage_pct(0.0, 0.0), 0.0);
	}

	#[test]
	fn usage_pct_idle_is_zero() {
		assert_eq!(usage_pct(100.0, 100.0), 0.0);
	}

	#[test]
	fn usage_pct_clamps() {
		assert_eq!(usage_pct(100.0, -10.0), 100.0);
		assert_eq!(usage_pct(0.0, 50.0), 0.0);
	}
}
