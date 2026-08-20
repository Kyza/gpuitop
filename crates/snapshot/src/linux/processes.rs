use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::time::Instant;

use gpuitop_core::model::*;
use gpuitop_gpu::build_vram_usage;

use super::proc_basics::username_for_uid;
use crate::{CollectorState, PrevProc};

impl CollectorState {
	#[hotpath::measure]
	pub fn collect_processes(
		&mut self,
		total_mem: u64,
		now: Instant,
	) -> Vec<ProcessSnapshot> {
		let Ok(procs) = procfs::process::all_processes() else {
			return Vec::new();
		};
		let num_cpus = gpuitop_core::cpu::cpu_count() as f32;

		let mut cur_proc_data: HashMap<
			i32,
			(u64, u64, u64, u64, bool, Option<String>, u64),
		> = HashMap::new();
		let mut pid_to_proc: HashMap<i32, ProcessSnapshot> = HashMap::new();
		let mut children_map: HashMap<i32, Vec<i32>> = HashMap::new();

		let vram_map = if self.gpu_data_enabled.load(Ordering::SeqCst) {
			build_vram_usage(&self.gpu_backends)
		} else {
			HashMap::new()
		};

		for proc_result in procs {
			let Ok(p) = proc_result else {
				continue;
			};
			let pid = p.pid;

			let Ok(basics) = super::proc_basics::ProcBasics::read(&p) else {
				continue;
			};
			let stat = &basics.stat;
			let status = &basics.status;
			let is_kthread = basics.is_kthread;
			let cmdline = &basics.cmdline;

			let cgroup = if is_kthread {
				String::new()
			} else {
				p.cgroups()
					.map(|cg| {
						cg.0.iter()
							.map(|c| {
								format!(
									"{}:{}:{}",
									c.hierarchy,
									c.controllers.join(","),
									c.pathname
								)
							})
							.collect::<Vec<_>>()
							.join("\n")
					})
					.unwrap_or_default()
			};

			// GUI status is fixed at exec time (DISPLAY/WAYLAND_DISPLAY presence
			// in the environment), so cache it across ticks. Reuse the previous
			// value only when starttime matches — that guards against PID reuse.
			let is_gui = if is_kthread {
				false
			} else {
				match self.prev_proc.get(&pid) {
					Some(prev) if prev.starttime == stat.starttime => {
						prev.is_gui
					}
					_ => p
						.environ()
						.map(|e| has_display_var_from_env(&e))
						.unwrap_or(false),
				}
			};

			let uid = status.ruid;
			let vmrss = status.vmrss.unwrap_or(0) * 1024;

			let command = cmdline.join(" ");
			let display_command = if command.is_empty() {
				basics.name()
			} else {
				command
			};

			let display_name = basics.name();

			// Icon name is fixed at exec time (desktop entry for the command),
			// so cache it across ticks like GUI status, guarded by starttime
			// against PID reuse. Kernel threads have no command to match.
			let icon_name = if is_kthread {
				None
			} else {
				match self.prev_proc.get(&pid) {
					Some(prev) if prev.starttime == stat.starttime => {
						prev.icon_name.clone()
					}
					_ => self
						.desktop_cache
						.lookup(&display_command, &display_name)
						.map(|de| de.icon_name.clone()),
				}
			};

			let is_owned = uid == self.current_uid;
			let user = self
				.user_cache
				.entry(uid)
				.or_insert_with(|| username_for_uid(uid))
				.clone();
			let mem_percent = if total_mem > 0 {
				vmrss as f32 / total_mem as f32 * 100.0
			} else {
				0.0
			};

			let vram = if is_kthread {
				VramUsage::default()
			} else {
				vram_map.get(&pid).copied().unwrap_or_default()
			};

			cur_proc_data.insert(
				pid,
				(
					stat.utime,
					stat.stime,
					stat.cutime as u64,
					stat.cstime as u64,
					is_gui,
					icon_name.clone(),
					stat.starttime,
				),
			);

			let cpu_percent = if let Some(ref prev_time) = self.prev_time {
				let elapsed =
					now.duration_since(*prev_time).as_secs_f32().max(0.001);
				if let Some(prev) = self.prev_proc.get(&pid) {
					let delta = basics.cpu_tick_sum().saturating_sub(
						prev.utime + prev.stime + prev.cutime + prev.cstime,
					);
					proc_cpu_pct(
						delta,
						elapsed,
						num_cpus,
						procfs::ticks_per_second() as f32,
					)
				} else {
					0.0
				}
			} else {
				0.0
			};

			children_map.entry(stat.ppid).or_default().push(pid);

			pid_to_proc.insert(
				pid,
				ProcessSnapshot {
					pid,
					ppid: stat.ppid,
					name: display_name,
					user,
					state: stat.state,
					command: display_command,
					cgroup,
					cpu_percent,
					mem_percent,
					mem_rss: vmrss,
					vram,
					disk_read_bytes_per_sec: 0.0,
					disk_write_bytes_per_sec: 0.0,
					is_gui,
					is_kthread,
					is_owned_by_current_user: is_owned,
					is_electron: false,
					electron_app_name: None,
					icon_name,
					has_children: false,
				},
			);
		}

		for (ppid, kids) in &children_map {
			if let Some(proc) = pid_to_proc.get_mut(ppid) {
				proc.has_children = !kids.is_empty();
			}
		}

		detect_electron_processes(&mut pid_to_proc);
		propagate_icons(&mut pid_to_proc);

		self.prev_proc = cur_proc_data
			.into_iter()
			.map(|(pid, (u, s, cu, cs, is_gui, icon_name, starttime))| {
				(
					pid,
					PrevProc {
						utime: u,
						stime: s,
						cutime: cu,
						cstime: cs,
						is_gui,
						icon_name,
						starttime,
					},
				)
			})
			.collect();

		pid_to_proc.into_values().collect()
	}
}

#[hotpath::measure]
fn detect_electron_processes(
	pid_to_proc: &mut HashMap<i32, ProcessSnapshot>,
) {
	let electron_subs: HashSet<i32> = pid_to_proc
		.iter()
		.filter(|(_, p)| p.command.contains("--type="))
		.map(|(pid, _)| *pid)
		.collect();

	if electron_subs.is_empty() {
		return;
	}

	let mut electron_pids: HashSet<i32> = electron_subs.clone();
	let mut root_map: HashMap<i32, i32> = HashMap::new();

	for &sub_pid in &electron_subs {
		let root = find_electron_root(sub_pid, &electron_subs, pid_to_proc);
		root_map.insert(sub_pid, root);

		let mut cur = sub_pid;
		while let Some(proc) = pid_to_proc.get(&cur) {
			electron_pids.insert(cur);
			if cur == root || proc.ppid == 0 || proc.ppid == cur {
				break;
			}
			cur = proc.ppid;
		}
	}

	let roots: HashSet<i32> = root_map.values().copied().collect();
	let real_electron_roots: HashSet<i32> = roots
		.iter()
		.filter(|r| {
			let exe = procfs::process::Process::new(**r)
				.ok()
				.and_then(|p| p.exe().ok())
				.unwrap_or_default();
			if exe.to_string_lossy().contains("electron") {
				return true;
			}
			electron_subs.iter().any(|sub| {
				root_map.get(sub) == Some(r)
					&& pid_to_proc
						.get(sub)
						.map(|p| p.command.contains(".asar"))
						.unwrap_or(false)
			})
		})
		.copied()
		.collect();

	for pid in &electron_pids {
		let root = root_map.get(pid).unwrap_or(pid);
		if real_electron_roots.contains(root) {
			if let Some(proc) = pid_to_proc.get_mut(pid) {
				proc.is_electron = true;
			}
		}
	}

	for (_sub_pid, root_pid) in &root_map {
		if let Some(proc) = pid_to_proc.get_mut(root_pid) {
			if proc.electron_app_name.is_none() {
				let app_name =
					extract_electron_app_name(&proc.command, &proc.name);
				proc.electron_app_name = Some(app_name);
			}
		}
	}
}

// Propagate icons down the process tree: a process without its own icon
// inherits the nearest ancestor's; a child that does have one overrides for
// its own subtree.
fn propagate_icons(pid_to_proc: &mut HashMap<i32, ProcessSnapshot>) {
	let mut children_map: HashMap<i32, Vec<i32>> = HashMap::new();
	for (pid, proc) in pid_to_proc.iter() {
		children_map.entry(proc.ppid).or_default().push(*pid);
	}
	let mut roots: Vec<i32> = pid_to_proc
		.keys()
		.filter(|pid| {
			let ppid = pid_to_proc.get(pid).map(|p| p.ppid).unwrap_or(0);
			ppid == 0 || !pid_to_proc.contains_key(&ppid)
		})
		.copied()
		.collect();
	roots.sort_unstable();

	let mut stack: Vec<(i32, Option<String>)> =
		roots.into_iter().map(|pid| (pid, None)).collect();
	while let Some((pid, inherited)) = stack.pop() {
		let own = pid_to_proc.get(&pid).and_then(|p| p.icon_name.clone());
		let effective = own.or(inherited);
		if let Some(proc) = pid_to_proc.get_mut(&pid) {
			if proc.icon_name.is_none() {
				proc.icon_name = effective.clone();
			}
		}
		if let Some(kids) = children_map.get(&pid) {
			for &kid in kids {
				stack.push((kid, effective.clone()));
			}
		}
	}
}

fn find_electron_root(
	pid: i32,
	electron_subs: &HashSet<i32>,
	pids: &HashMap<i32, ProcessSnapshot>,
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

// Per-process CPU percent: delta ticks over the elapsed window, normalized by
// hertz and core count, clamped to [0, 100].
fn proc_cpu_pct(
	delta_ticks: u64,
	elapsed: f32,
	num_cpus: f32,
	hertz: f32,
) -> f32 {
	(delta_ticks as f32 / hertz / elapsed / num_cpus * 100.0)
		.clamp(0.0, 100.0)
}

#[hotpath::measure]
fn has_display_var_from_env(
	environ: &std::collections::HashMap<
		std::ffi::OsString,
		std::ffi::OsString,
	>,
) -> bool {
	environ.contains_key(std::ffi::OsStr::new("DISPLAY"))
		|| environ.contains_key(std::ffi::OsStr::new("WAYLAND_DISPLAY"))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::collect_snapshot;
	use std::sync::atomic::AtomicBool;
	use std::sync::Arc;

	fn test_state() -> CollectorState {
		CollectorState::new(
			vec![],
			Default::default(),
			Arc::new(AtomicBool::new(true)),
			Arc::new(AtomicBool::new(false)),
		)
	}

	#[test]
	fn test_collector_has_processes() {
		let mut state = test_state();
		let snap = collect_snapshot(&mut state);

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
				proc.vram
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
		let mut state = test_state();

		collect_snapshot(&mut state);
		let snap = collect_snapshot(&mut state);

		assert!(!snap.cpu.cores.is_empty(), "No CPU cores detected");
		eprintln!(
			"CPU cores: {}, overall: {:.1}%",
			snap.cpu.cores.len(),
			snap.cpu.overall_percent
		);
	}

	#[test]
	fn test_collector_has_memory_data() {
		let mut state = test_state();
		let snap = collect_snapshot(&mut state);

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
		let mut state = test_state();
		let snap = collect_snapshot(&mut state);

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
	fn proc_cpu_pct_half_core() {
		// 1s of a single core = hertz ticks over a 1s window, 1 core.
		let hertz = 100.0;
		let pct = proc_cpu_pct(hertz as u64, 1.0, 1.0, hertz);
		assert_eq!(pct, 100.0);
		assert_eq!(proc_cpu_pct(hertz as u64 / 2, 1.0, 1.0, hertz), 50.0);
	}

	#[test]
	fn proc_cpu_pct_normalizes_by_cores_and_time() {
		// Same ticks but 8 cores and a 2s window → 1/16 of full.
		let pct = proc_cpu_pct(100, 2.0, 8.0, 100.0);
		assert!((pct - 6.25).abs() < 0.001);
	}

	#[test]
	fn proc_cpu_pct_clamps_at_100() {
		assert_eq!(proc_cpu_pct(1000, 1.0, 1.0, 100.0), 100.0);
		assert_eq!(proc_cpu_pct(0, 1.0, 1.0, 100.0), 0.0);
	}

	#[test]
	fn test_extract_electron_app_name_non_electron() {
		let name = extract_electron_app_name(
			"/usr/bin/firefox --new-window",
			"firefox",
		);
		assert_eq!(name, "firefox");
	}

	#[test]
	fn test_extract_electron_app_name_from_path() {
		let name = extract_electron_app_name(
			"/opt/Discord/discord --type=renderer",
			"electron",
		);
		assert_eq!(name, "Discord");
	}

	#[test]
	fn test_extract_electron_app_name_skips_flags() {
		let name = extract_electron_app_name(
			"/usr/share/code/code --no-sandbox --type=renderer",
			"electron",
		);
		assert_eq!(name, "Code");
	}

	#[test]
	fn test_extract_electron_app_name_skips_electron_dir() {
		let name = extract_electron_app_name(
			"/opt/Spotify/electron/spotify --type=renderer",
			"electron",
		);
		assert_eq!(name, "Spotify");
	}

	#[test]
	fn test_extract_electron_app_name_skips_node_modules() {
		let name = extract_electron_app_name(
			"/usr/share/code --type=renderer",
			"electron",
		);
		assert_eq!(name, "Code");
	}

	#[test]
	fn test_extract_electron_app_name_fallback_to_binary() {
		let name = extract_electron_app_name(
			"/opt/electron-app/electron-bin --type=renderer",
			"electron",
		);
		assert_eq!(name, "Electron-bin");
	}

	#[test]
	fn test_extract_electron_app_name_fallback_to_name() {
		let name = extract_electron_app_name(
			"--type=renderer --some-flag",
			"electron-something",
		);
		assert_eq!(name, "electron-something");
	}

	#[test]
	fn test_capitalize() {
		assert_eq!(capitalize("hello"), "Hello");
		assert_eq!(capitalize("HELLO"), "Hello");
		assert_eq!(capitalize("h"), "H");
		assert_eq!(capitalize(""), "");
		assert_eq!(capitalize("discord"), "Discord");
	}

	fn make_stub_proc(pid: i32, ppid: i32) -> ProcessSnapshot {
		ProcessSnapshot {
			pid,
			ppid,
			name: String::new(),
			user: String::new(),
			state: 'S',
			command: String::new(),
			cgroup: String::new(),
			cpu_percent: 0.0,
			mem_percent: 0.0,
			mem_rss: 0,
			vram: VramUsage::default(),
			disk_read_bytes_per_sec: 0.0,
			disk_write_bytes_per_sec: 0.0,
			is_gui: false,
			is_kthread: false,
			is_owned_by_current_user: false,
			is_electron: false,
			electron_app_name: None,
			icon_name: None,
			has_children: false,
		}
	}

	#[test]
	fn test_propagate_icons_inherits_parent_icon() {
		let root = ProcessSnapshot {
			icon_name: Some("firefox".into()),
			..make_stub_proc(1, 0)
		};
		let child = make_stub_proc(2, 1);
		let grandchild = make_stub_proc(3, 2);
		let mut pids: HashMap<i32, ProcessSnapshot> =
			[(1, root), (2, child), (3, grandchild)].into();
		propagate_icons(&mut pids);
		assert_eq!(pids[&1].icon_name.as_deref(), Some("firefox"));
		assert_eq!(pids[&2].icon_name.as_deref(), Some("firefox"));
		assert_eq!(pids[&3].icon_name.as_deref(), Some("firefox"));
	}

	#[test]
	fn test_propagate_icons_child_overrides_subtree() {
		let root = ProcessSnapshot {
			icon_name: Some("bash".into()),
			..make_stub_proc(1, 0)
		};
		let child = ProcessSnapshot {
			icon_name: Some("gnome-terminal".into()),
			..make_stub_proc(2, 1)
		};
		let grandchild = make_stub_proc(3, 2);
		let mut pids: HashMap<i32, ProcessSnapshot> =
			[(1, root), (2, child), (3, grandchild)].into();
		propagate_icons(&mut pids);
		assert_eq!(pids[&1].icon_name.as_deref(), Some("bash"));
		assert_eq!(pids[&2].icon_name.as_deref(), Some("gnome-terminal"));
		assert_eq!(pids[&3].icon_name.as_deref(), Some("gnome-terminal"));
	}

	#[test]
	fn test_propagate_icons_orphan_keeps_own() {
		let orphan = ProcessSnapshot {
			icon_name: Some("vlc".into()),
			..make_stub_proc(9, 999)
		};
		let mut pids: HashMap<i32, ProcessSnapshot> = [(9, orphan)].into();
		propagate_icons(&mut pids);
		assert_eq!(pids[&9].icon_name.as_deref(), Some("vlc"));
	}

	#[test]
	fn test_propagate_icons_keeps_existing_icons() {
		let root = ProcessSnapshot {
			icon_name: Some("a".into()),
			..make_stub_proc(1, 0)
		};
		let child = ProcessSnapshot {
			icon_name: Some("b".into()),
			..make_stub_proc(2, 1)
		};
		let mut pids: HashMap<i32, ProcessSnapshot> =
			[(1, root), (2, child)].into();
		propagate_icons(&mut pids);
		assert_eq!(pids[&1].icon_name.as_deref(), Some("a"));
		assert_eq!(pids[&2].icon_name.as_deref(), Some("b"));
	}

	#[test]
	fn test_find_electron_root_direct_parent() {
		let sub = ProcessSnapshot {
			pid: 200,
			ppid: 100,
			name: "electron".into(),
			command: "--type=renderer".into(),
			..make_stub_proc(200, 100)
		};
		let parent = ProcessSnapshot {
			pid: 100,
			ppid: 1,
			name: "discord".into(),
			command: "/opt/Discord/discord".into(),
			..make_stub_proc(100, 1)
		};
		let pids: HashMap<i32, ProcessSnapshot> =
			[(200, sub), (100, parent)].into();
		let subs: HashSet<i32> = [200].into();
		let root = find_electron_root(200, &subs, &pids);
		assert_eq!(root, 100);
	}

	#[test]
	fn test_find_electron_root_nested_subprocesses() {
		let deepest = ProcessSnapshot {
			pid: 300,
			ppid: 200,
			name: "electron".into(),
			command: "--type=renderer".into(),
			..make_stub_proc(300, 200)
		};
		let mid = ProcessSnapshot {
			pid: 200,
			ppid: 100,
			name: "electron".into(),
			command: "--type=zygote".into(),
			..make_stub_proc(200, 100)
		};
		let root = ProcessSnapshot {
			pid: 100,
			ppid: 1,
			name: "code".into(),
			command: "/usr/share/code/code".into(),
			..make_stub_proc(100, 1)
		};
		let pids: HashMap<i32, ProcessSnapshot> =
			[(300, deepest), (200, mid), (100, root)].into();
		let subs: HashSet<i32> = [300, 200].into();
		let r = find_electron_root(300, &subs, &pids);
		assert_eq!(r, 100);
	}

	#[test]
	fn test_find_electron_root_pid_not_found() {
		let subs: HashSet<i32> = [999].into();
		let pids: HashMap<i32, ProcessSnapshot> = HashMap::new();
		let r = find_electron_root(999, &subs, &pids);
		assert_eq!(r, 999);
	}
}
