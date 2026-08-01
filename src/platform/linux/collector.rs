use crate::model::*;
use std::collections::HashMap;
use std::fs;
use std::time::Instant;

const HISTORY_LEN: usize = 60;

pub struct SystemCollector {
	pub(crate) gpu_backend: GpuBackend,
	pub(crate) current_uid: u32,
	pub(crate) user_cache: HashMap<u32, String>,
	prev_cpu_totals: Option<Vec<(u64, u64)>>,
	prev_cpu_time: Option<Instant>,
	prev_disk: HashMap<String, (u64, u64)>,
	prev_disk_time: Option<Instant>,
	prev_net: HashMap<String, (u64, u64)>,
	prev_net_time: Option<Instant>,
	pub(crate) prev_proc: HashMap<i32, (u64, u64, u64, u64)>,
	pub(crate) prev_proc_time: Option<Instant>,
	core_history: HashMap<usize, Vec<f32>>,
}

impl SystemCollector {
	pub fn new(gpu_backend: GpuBackend) -> Self {
		Self {
			gpu_backend,
			current_uid: unsafe { libc::getuid() },
			user_cache: HashMap::new(),
			prev_cpu_totals: None,
			prev_cpu_time: None,
			prev_disk: HashMap::new(),
			prev_disk_time: None,
			prev_net: HashMap::new(),
			prev_net_time: None,
			prev_proc: HashMap::new(),
			prev_proc_time: None,
			core_history: HashMap::new(),
		}
	}

	#[hotpath::measure]
	pub fn tick(&mut self) -> SystemSnapshot {
		let now = Instant::now();
		let cpu = self.collect_cpu(now);
		let memory = self.collect_memory();
		let total_mem = memory.total.max(1);
		let disks = self.collect_disks(now);
		let networks = self.collect_networks(now);
		let processes = self.collect_processes(now, total_mem);

		SystemSnapshot {
			processes,
			cpu,
			memory,
			disks,
			networks,
			timestamp: now,
			gpu_backend: self.gpu_backend,
		}
	}

	pub(crate) fn read(path: &str) -> Option<String> {
		fs::read_to_string(path).ok()
	}

	pub(crate) fn parse_kv(data: &str) -> HashMap<String, String> {
		let mut map = HashMap::new();
		for line in data.lines() {
			if let Some((k, v)) = line.split_once(':') {
				map.insert(k.trim().to_string(), v.trim().to_string());
			}
		}
		map
	}

	// ---- CPU ----
	#[hotpath::measure]
	fn collect_cpu(&mut self, now: Instant) -> CpuInfo {
		let Some(data) = Self::read("/proc/stat") else {
			return CpuInfo {
				cores: Vec::new(),
				overall_percent: 0.0,
			};
		};

		let mut cur_totals: Vec<(u64, u64)> = Vec::new();

		for line in data.lines() {
			if !line.starts_with("cpu") {
				continue;
			}
			let parts: Vec<u64> = line
				.split_whitespace()
				.skip(1)
				.filter_map(|v| v.parse().ok())
				.collect();
			if parts.len() < 5 {
				continue;
			}
			let total: u64 = parts.iter().sum();
			let idle: u64 =
				parts.get(3).unwrap_or(&0) + parts.get(4).unwrap_or(&0);
			cur_totals.push((total, idle));
		}

		let mut cores = Vec::new();

		if let (Some(ref prev), Some(ref _prev_time)) =
			(&self.prev_cpu_totals, &self.prev_cpu_time)
		{
			for (i, (total, idle)) in cur_totals.iter().enumerate() {
				if i >= prev.len() {
					continue;
				}
				let (prev_total, prev_idle) = prev[i];
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
						history: Vec::new(),
					});
				} else {
					let cidx = i - 1;
					let history = self.core_history.entry(cidx).or_default();
					history.push(usage);
					if history.len() > HISTORY_LEN {
						history.remove(0);
					}
					cores.push(CpuCore {
						index: cidx,
						usage_percent: usage,
						history: history.clone(),
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
					history: Vec::new(),
				});
			}
		}

		// First entry is aggregate "cpu", rest are per-core
		let overall = cores.first().map(|c| c.usage_percent).unwrap_or(0.0);

		self.prev_cpu_totals = Some(cur_totals);
		self.prev_cpu_time = Some(now);

		CpuInfo {
			cores,
			overall_percent: overall,
		}
	}

	// ---- Memory ----
	#[hotpath::measure]
	fn collect_memory(&self) -> MemoryInfo {
		let data = Self::read("/proc/meminfo").unwrap_or_default();
		let map = Self::parse_kv(&data);

		fn get(map: &HashMap<String, String>, key: &str) -> u64 {
			map.get(key)
				.and_then(|v| v.split_whitespace().next())
				.and_then(|n| n.parse().ok())
				.unwrap_or(0)
				* 1024 // meminfo reports in kB, we want bytes
		}

		let total = get(&map, "MemTotal");
		let free = get(&map, "MemFree");
		let buffers = get(&map, "Buffers");
		let cached = get(&map, "Cached") + get(&map, "SReclaimable");
		let available = get(&map, "MemAvailable");
		let used = total.saturating_sub(free + buffers + cached);
		let swap_total = get(&map, "SwapTotal");
		let swap_free = get(&map, "SwapFree");

		MemoryInfo {
			total,
			used,
			available,
			free,
			buffers,
			cached,
			swap_total,
			swap_free,
			swap_used: swap_total.saturating_sub(swap_free),
		}
	}

	// ---- Disk ----
	fn collect_disks(&mut self, now: Instant) -> Vec<DiskInfo> {
		let data = Self::read("/proc/diskstats").unwrap_or_default();
		let mut cur: HashMap<String, (u64, u64)> = HashMap::new();
		for line in data.lines() {
			let parts: Vec<&str> = line.split_whitespace().collect();
			if parts.len() < 14 {
				continue;
			}
			let name = parts[2].to_string();
			let read_sectors: u64 = parts[5].parse().unwrap_or(0);
			let write_sectors: u64 = parts[9].parse().unwrap_or(0);
			cur.insert(name, (read_sectors * 512, write_sectors * 512));
		}

		let mut result = Vec::new();
		if let Some(ref prev_time) = self.prev_disk_time {
			let elapsed =
				now.duration_since(*prev_time).as_secs_f64().max(0.001);
			for (name, (cr, cw)) in &cur {
				if let Some((pr, pw)) = self.prev_disk.get(name) {
					result.push(DiskInfo {
						device: name.clone(),
						read_bytes_per_sec: cr.saturating_sub(*pr) as f64
							/ elapsed,
						write_bytes_per_sec: cw.saturating_sub(*pw) as f64
							/ elapsed,
					});
				}
			}
		}
		self.prev_disk = cur;
		self.prev_disk_time = Some(now);
		result
	}

	// ---- Network ----
	fn collect_networks(&mut self, now: Instant) -> Vec<NetInfo> {
		let data = Self::read("/proc/net/dev").unwrap_or_default();
		let mut cur: HashMap<String, (u64, u64)> = HashMap::new();
		for line in data.lines().skip(2) {
			let parts: Vec<&str> = line.split_whitespace().collect();
			if parts.len() < 10 {
				continue;
			}
			let name = parts[0].trim_end_matches(':').to_string();
			let rx: u64 = parts[1].parse().unwrap_or(0);
			let tx: u64 = parts[9].parse().unwrap_or(0);
			cur.insert(name, (rx, tx));
		}

		let mut result = Vec::new();
		if let Some(ref prev_time) = self.prev_net_time {
			let elapsed =
				now.duration_since(*prev_time).as_secs_f64().max(0.001);
			for (name, (crx, ctx)) in &cur {
				if let Some((prx, ptx)) = self.prev_net.get(name) {
					result.push(NetInfo {
						interface: name.clone(),
						rx_bytes_per_sec: crx.saturating_sub(*prx) as f64
							/ elapsed,
						tx_bytes_per_sec: ctx.saturating_sub(*ptx) as f64
							/ elapsed,
					});
				}
			}
		}
		self.prev_net = cur;
		self.prev_net_time = Some(now);
		result
	}
}
