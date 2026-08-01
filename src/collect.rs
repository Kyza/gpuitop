use crate::model::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::time::Instant;

const HISTORY_LEN: usize = 60;

pub struct SystemCollector {
	gpu_backend: GpuBackend,
	current_uid: u32,
	user_cache: HashMap<u32, String>,
	prev_cpu_totals: Option<Vec<(u64, u64)>>,
	prev_cpu_time: Option<Instant>,
	prev_disk: HashMap<String, (u64, u64)>,
	prev_disk_time: Option<Instant>,
	prev_net: HashMap<String, (u64, u64)>,
	prev_net_time: Option<Instant>,
	prev_proc: HashMap<i32, (u64, u64, u64, u64)>,
	prev_proc_time: Option<Instant>,
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

	fn read(path: &str) -> Option<String> {
		fs::read_to_string(path).ok()
	}

	fn parse_kv(data: &str) -> HashMap<String, String> {
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

	#[hotpath::measure]
	fn parse_stat(
		stat_data: &str,
	) -> Option<(String, char, i32, u64, u64, u64, u64)> {
		let comm_start = stat_data.find('(')? + 1;
		let comm_end = stat_data.find(')')?;
		let comm = stat_data[comm_start..comm_end].to_string();
		let rest = &stat_data[comm_end + 2..];
		let parts: Vec<&str> = rest.split_whitespace().collect();
		if parts.len() < 15 {
			return None;
		}

		let state = parts[0].chars().next()?;
		let ppid: i32 = parts[1].parse().ok()?;
		let utime: u64 = parts[11].parse().ok()?;
		let stime: u64 = parts[12].parse().ok()?;
		let cutime: u64 = parts[13].parse().ok()?;
		let cstime: u64 = parts[14].parse().ok()?;

		Some((comm, state, ppid, utime, stime, cutime, cstime))
	}

	// ---- Processes ----
	#[hotpath::measure]
	fn collect_processes(
		&mut self,
		now: Instant,
		total_mem: u64,
	) -> Vec<ProcessInfo> {
		let Ok(entries) = fs::read_dir("/proc") else {
			return Vec::new();
		};
		let num_cpus = num_cpus::get() as f32;

		let mut cur_proc_data: HashMap<i32, (u64, u64, u64, u64)> =
			HashMap::new();
		let mut pid_to_proc: HashMap<i32, ProcessInfo> = HashMap::new();
		let mut children_map: HashMap<i32, Vec<i32>> = HashMap::new();

		// Build VRAM map once instead of querying per-PID
		let vram_map = build_vram_map(self.gpu_backend);

		for entry in entries.flatten() {
			let name_str = entry.file_name().to_string_lossy().to_string();
			let pid: i32 = match name_str.parse() {
				Ok(p) => p,
				Err(_) => continue,
			};
			let base = format!("/proc/{pid}");

			// Read stat
			let stat_data =
				Self::read(&format!("{base}/stat")).unwrap_or_default();
			let Some((proc_name, state, ppid, utime, stime, cutime, cstime)) =
				Self::parse_stat(&stat_data)
			else {
				continue;
			};

			// Read status
			let status_data =
				Self::read(&format!("{base}/status")).unwrap_or_default();
			let status_map = Self::parse_kv(&status_data);

			let uid: u32 = status_map
				.get("Uid")
				.and_then(|v| v.split_whitespace().next())
				.and_then(|s| s.parse().ok())
				.unwrap_or(0);

			let vmrss: u64 = status_map
				.get("VmRSS")
				.and_then(|v| v.split_whitespace().next())
				.and_then(|s| s.parse::<u64>().ok())
				.unwrap_or(0)
				* 1024;

			// Command line
			let command = Self::read(&format!("{base}/cmdline"))
				.unwrap_or_default()
				.replace('\0', " ")
				.trim()
				.to_string();

			let display_command = if command.is_empty() {
				proc_name.clone()
			} else {
				command
			};

			// I/O
			let io_data =
				Self::read(&format!("{base}/io")).unwrap_or_default();
			let io_map = Self::parse_kv(&io_data);
			let _read_bytes: u64 = io_map
				.get("read_bytes")
				.and_then(|v| v.parse().ok())
				.unwrap_or(0);
			let _write_bytes: u64 = io_map
				.get("write_bytes")
				.and_then(|v| v.parse().ok())
				.unwrap_or(0);

			let is_kthread = ppid == 2 || proc_name.starts_with('[');
			let is_gui = if is_kthread {
				false
			} else {
				has_display_var(pid)
			};
			let is_owned = uid == self.current_uid;
			let user = self.user_cache
				.entry(uid)
				.or_insert_with(|| get_user_name(uid))
				.clone();
			let mem_percent = if total_mem > 0 {
				vmrss as f32 / total_mem as f32 * 100.0
			} else {
				0.0
			};

			// VRAM — skip kernel threads, look up from precomputed map
			let vram_bytes = if is_kthread {
				None
			} else {
				vram_map.get(&pid).copied()
			};

			cur_proc_data.insert(pid, (utime, stime, cutime, cstime));

			let cpu_percent = if let Some(ref prev_time) = self.prev_proc_time
			{
				let elapsed =
					now.duration_since(*prev_time).as_secs_f32().max(0.001);
				if let Some((pu, ps, pcu, pcs)) = self.prev_proc.get(&pid) {
					let delta = (utime + stime + cutime + cstime)
						.saturating_sub(pu + ps + pcu + pcs)
						as f32;
					let hertz = ticks_per_second() as f32;
					(delta / hertz / elapsed / num_cpus * 100.0)
						.clamp(0.0, 100.0)
				} else {
					0.0
				}
			} else {
				0.0
			};

			children_map.entry(ppid).or_default().push(pid);

			pid_to_proc.insert(
				pid,
				ProcessInfo {
					pid,
					ppid,
					name: proc_name,
					user,
					state,
					command: display_command,
					cpu_percent,
					mem_percent,
					mem_rss: vmrss,
					vram_bytes,
					disk_read_bytes_per_sec: 0.0,
					disk_write_bytes_per_sec: 0.0,
					is_gui,
					is_kthread,
					is_owned_by_current_user: is_owned,
					is_electron: false,
					electron_app_name: None,
					children: Vec::new(),
					has_children: false,
				},
			);
		}

		// Mark which pids have children
		for (ppid, kids) in &children_map {
			if let Some(proc) = pid_to_proc.get_mut(ppid) {
				proc.has_children = !kids.is_empty();
			}
		}

		// Electron detection: mark processes in electron trees
		hotpath::measure_block!("electron_detect", {
			// Pass 1: find processes with --type= in cmdline (electron subprocesses)
			let electron_subs: HashSet<i32> = pid_to_proc
				.iter()
				.filter(|(_, p)| p.command.contains("--type="))
				.map(|(pid, _)| *pid)
				.collect();

			if !electron_subs.is_empty() {
				// Pass 2: walk up ppid chain from each electron subprocess,
				// marking ancestors as electron, recording root pid
				let mut electron_pids: HashSet<i32> = electron_subs.clone();
				let mut root_map: HashMap<i32, i32> = HashMap::new(); // child -> root pid

				for &sub_pid in &electron_subs {
					let root = find_electron_root(
						sub_pid,
						&electron_subs,
						&pid_to_proc,
					);
					root_map.insert(sub_pid, root);

					// Mark all ancestors up to root
					let mut cur = sub_pid;
					while let Some(proc) = pid_to_proc.get(&cur) {
						electron_pids.insert(cur);
						if cur == root || proc.ppid == 0 || proc.ppid == cur {
							break;
						}
						cur = proc.ppid;
					}
				}

				// Set is_electron on all tree processes
				for pid in &electron_pids {
					if let Some(proc) = pid_to_proc.get_mut(pid) {
						proc.is_electron = true;
					}
				}

				// Set electron_app_name on root processes
				for (_sub_pid, root_pid) in &root_map {
					if let Some(proc) = pid_to_proc.get_mut(root_pid) {
						if proc.electron_app_name.is_none() {
							let app_name = extract_electron_app_name(
								&proc.command,
								&proc.name,
							);
							proc.electron_app_name = Some(app_name);
						}
					}
				}
			}
		});

		self.prev_proc = cur_proc_data;
		self.prev_proc_time = Some(now);

		pid_to_proc.into_values().collect()
	}
}

fn ticks_per_second() -> u64 {
	unsafe { libc::sysconf(libc::_SC_CLK_TCK) as u64 }
}

fn find_electron_root(
	pid: i32,
	electron_subs: &HashSet<i32>,
	pids: &HashMap<i32, ProcessInfo>,
) -> i32 {
	let mut current = pid;
	loop {
		let proc = match pids.get(&current) {
			Some(p) => p,
			None => break pid,
		};
		let ppid = proc.ppid;
		if ppid == 0 || ppid == current {
			break current;
		}
		let parent = match pids.get(&ppid) {
			Some(p) => p,
			None => break current,
		};
		// If parent is NOT an electron subprocess, it's the root
		if !electron_subs.contains(&ppid)
			&& !parent.command.contains("--type=")
		{
			break ppid;
		}
		current = ppid;
	}
}

fn extract_electron_app_name(cmdline: &str, name: &str) -> String {
	if !name.to_lowercase().contains("electron") {
		return name.to_string();
	}
	let parts: Vec<&str> = cmdline.split_whitespace().collect();
	for part in &parts[1..] {
		if part.starts_with('-') {
			continue;
		}
		if part.contains('/') {
			if let Some(parent_dir) = std::path::Path::new(part)
				.parent()
				.and_then(|p| p.file_name())
				.and_then(|n| n.to_str())
			{
				let lower = parent_dir.to_lowercase();
				if !lower.contains("electron") && lower != "node_modules" {
					return capitalize(parent_dir);
				}
			}
			if let Some(stem) = std::path::Path::new(part)
				.file_stem()
				.and_then(|n| n.to_str())
			{
				let lower = stem.to_lowercase();
				if !lower.contains("electron") {
					return capitalize(stem);
				}
			}
		}
	}
	match parts.first() {
		Some(bin) if bin.contains('/') => {
			if let Some(n) = std::path::Path::new(bin)
				.file_name()
				.and_then(|n| n.to_str())
			{
				return capitalize(n);
			}
		}
		_ => {}
	}
	name.to_string()
}

fn capitalize(s: &str) -> String {
	let mut chars = s.chars();
	match chars.next() {
		None => String::new(),
		Some(c) => {
			let mut out = c.to_uppercase().to_string();
			out.push_str(&chars.collect::<String>().to_lowercase());
			out
		}
	}
}

#[hotpath::measure]
fn has_display_var(pid: i32) -> bool {
	if let Ok(data) = fs::read(format!("/proc/{pid}/environ")) {
		return data.split(|&b| b == 0).any(|s| {
			s.starts_with(b"DISPLAY=") || s.starts_with(b"WAYLAND_DISPLAY=")
		});
	}
	false
}

#[hotpath::measure]
fn get_user_name(uid: u32) -> String {
	users::get_user_by_uid(uid)
		.map(|u| u.name().to_string_lossy().to_string())
		.unwrap_or_else(|| uid.to_string())
}

#[hotpath::measure]
fn build_vram_map(gpu_backend: GpuBackend) -> HashMap<i32, u64> {
	match gpu_backend {
		GpuBackend::Nvidia => build_nvidia_vram_map(),
		GpuBackend::Amd => build_rocm_vram_map(),
		GpuBackend::None => HashMap::new(),
	}
}

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
		let pid: i32 = match parts.next().and_then(|s| s.trim().parse().ok()) {
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

pub fn detect_gpu() -> GpuBackend {
	if nvml_wrapper::Nvml::init().is_ok() {
		return GpuBackend::Nvidia;
	}
	if std::process::Command::new("rocm-smi")
		.arg("--showpids")
		.output()
		.map(|o| o.status.success())
		.unwrap_or(false)
	{
		return GpuBackend::Amd;
	}
	GpuBackend::None
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_collector_has_processes() {
		let mut collector = SystemCollector::new(GpuBackend::None);
		let snap = collector.tick();

		assert!(
			!snap.processes.is_empty(),
			"No processes collected. Expected at least PID 1 (systemd/init)"
		);

		let init = snap.processes.iter().find(|p| p.pid == 1);
		assert!(init.is_some(), "PID 1 (init/systemd) not found");

		for proc in snap.processes.iter().take(10) {
			eprintln!(
				"PID={} name={} ppid={} gui={} user={} state={} cpu={:.1}% \
				 mem={:.1}% vram={:?}",
				proc.pid,
				proc.name,
				proc.ppid,
				proc.is_gui,
				proc.user,
				proc.state,
				proc.cpu_percent,
				proc.mem_percent,
				proc.vram_bytes
			);
		}

		eprintln!(
			"Collected {} processes, {} GUI",
			snap.processes.len(),
			snap.processes.iter().filter(|p| p.is_gui).count()
		);
	}

	#[test]
	fn test_collector_has_cpu_data() {
		let mut collector = SystemCollector::new(GpuBackend::None);

		collector.tick();
		let snap = collector.tick();

		assert!(!snap.cpu.cores.is_empty(), "No CPU cores detected");
		eprintln!(
			"CPU cores: {}, overall: {:.1}%",
			snap.cpu.cores.len(),
			snap.cpu.overall_percent
		);
	}

	#[test]
	fn test_collector_has_memory_data() {
		let mut collector = SystemCollector::new(GpuBackend::None);
		let snap = collector.tick();

		assert!(snap.memory.total > 0, "Total memory is 0");
		eprintln!(
			"Memory: total={} used={} avail={} swap_used={}",
			snap.memory.total,
			snap.memory.used,
			snap.memory.available,
			snap.memory.swap_used
		);
	}

	#[test]
	fn test_processes_flat_list() {
		let mut collector = SystemCollector::new(GpuBackend::None);
		let snap = collector.tick();

		let total = snap.processes.len();
		eprintln!("Flat list: {} processes", total);
		assert!(total > 10, "Too few processes: {}", total);

		let with_children =
			snap.processes.iter().filter(|p| p.has_children).count();
		eprintln!("  with children: {}", with_children);
		assert!(with_children > 0, "No processes have has_children=true");

		let gui = snap.processes.iter().filter(|p| p.is_gui).count();
		eprintln!("  GUI processes: {}", gui);

		assert!(snap.processes.iter().any(|p| p.pid == 1), "PID 1 missing");
	}

	#[test]
	fn test_gui_detection() {
		let entries = std::fs::read_dir("/proc").unwrap();
		let mut gui_count = 0;

		for entry in entries.flatten() {
			let name = entry.file_name().to_string_lossy().to_string();
			let pid: i32 = match name.parse() {
				Ok(p) => p,
				_ => continue,
			};

			if let Ok(data) = std::fs::read(format!("/proc/{pid}/environ")) {
				let has_display = data
					.split(|&b| b == 0)
					.any(|s| s.starts_with(b"DISPLAY="));
				let has_wayland = data
					.split(|&b| b == 0)
					.any(|s| s.starts_with(b"WAYLAND_DISPLAY="));

				if has_display || has_wayland {
					gui_count += 1;
					if gui_count <= 5 {
						if let Ok(stat) = std::fs::read_to_string(format!(
							"/proc/{pid}/stat"
						)) {
							let sname = stat
								.split_whitespace()
								.nth(1)
								.map(|s| {
									s.trim_matches(|c| c == '(' || c == ')')
								})
								.unwrap_or("?");
							eprintln!("GUI: PID={pid} name={sname}");
						}
					}
				}
			}
		}

		eprintln!("Found {gui_count} GUI processes");
		assert!(
			gui_count > 0,
			"No GUI processes found. Running under a display server?"
		);
	}

	#[test]
	fn test_ppid1_processes() {
		let entries = std::fs::read_dir("/proc").unwrap();
		let mut systemd_count = 0;

		for entry in entries.flatten() {
			let name = entry.file_name().to_string_lossy().to_string();
			let pid: i32 = match name.parse() {
				Ok(p) => p,
				_ => continue,
			};

			if let Ok(stat) =
				std::fs::read_to_string(format!("/proc/{pid}/stat"))
			{
				let parts: Vec<&str> = stat.split_whitespace().collect();
				if parts.len() >= 4 {
					if let Ok(ppid) = parts[3].parse::<i32>() {
						if ppid == 1 {
							systemd_count += 1;
						}
					}
				}
			}
		}

		eprintln!("Found {systemd_count} processes with PPID=1");
	}

	#[test]
	fn bench_collector_tick() {
		let mut collector = SystemCollector::new(GpuBackend::None);

		// Warmup
		collector.tick();

		let iterations = 10;
		let start = std::time::Instant::now();
		for _ in 0..iterations {
			collector.tick();
		}
		let elapsed = start.elapsed();

		let avg = elapsed / iterations;
		eprintln!(
			"collector.tick() benchmark: {} iterations, total={:?}, avg={:?}",
			iterations, elapsed, avg
		);

		let snap = collector.tick();
		eprintln!("  processes per tick: {}", snap.processes.len());
		eprintln!(
			"  total={:?} avg={:?} ({} µs/proc)",
			elapsed,
			avg,
			avg.as_micros() as f64 / snap.processes.len() as f64
		);

		assert!(avg.as_millis() < 1000, "tick() too slow: {:?}", avg);
	}
}
