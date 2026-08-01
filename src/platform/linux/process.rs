use crate::model::*;
use super::collector::SystemCollector;
use super::gpu::build_vram_map;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::time::Instant;

impl SystemCollector {
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
	pub(crate) fn collect_processes(
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

			// Command line — raw with \0 separators
			let raw_cmdline =
				Self::read(&format!("{base}/cmdline")).unwrap_or_default();

			let cmdline_name = raw_cmdline
				.split('\0')
				.next()
				.filter(|a| !a.is_empty())
				.and_then(|argv0| std::path::Path::new(argv0).file_name())
				.and_then(|n| n.to_str())
				.map(|s| s.to_string());

			let command = raw_cmdline.replace('\0', " ").trim().to_string();

			let display_command = if command.is_empty() {
				cmdline_name.clone().unwrap_or_else(|| proc_name.clone())
			} else {
				command
			};

			let display_name =
				cmdline_name.unwrap_or_else(|| proc_name.clone());

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
					name: display_name,
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
