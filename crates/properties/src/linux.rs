use super::{
	LimitEntry, Platform, ProcessProperties, PropertiesInterface,
	SmapsSummary,
};
use procfs::process::LimitValue;

use gpuitop_core::model;

fn limit_to_str(limit: &LimitValue) -> String {
	match limit {
		LimitValue::Unlimited => "unlimited".to_string(),
		LimitValue::Value(v) => format!("{}", v),
	}
}

impl_interface! {
	fn collect(pid: i32) -> Option<ProcessProperties> {
	let proc = procfs::process::Process::new(pid).ok()?;
	let stat = proc.stat().ok()?;
	let status = proc.status().ok()?;

	let hertz = procfs::ticks_per_second() as f64;

	let uid = status.ruid;
	let username = users::get_user_by_uid(uid)
		.map(|u| u.name().to_string_lossy().into_owned())
		.unwrap_or_else(|| uid.to_string());

	let command = proc.cmdline().unwrap_or_default();
	let exe_path = proc.exe().ok().map(|p| p.to_string_lossy().into_owned());
	let cwd = proc.cwd().ok().map(|p| p.to_string_lossy().into_owned());
	let root_path =
		proc.root().ok().map(|p| p.to_string_lossy().into_owned());

	let environ = proc.environ().unwrap_or_default();
	let environ: Vec<(String, String)> = environ
		.into_iter()
		.map(|(k, v)| {
			(
				k.to_string_lossy().into_owned(),
				v.to_string_lossy().into_owned(),
			)
		})
		.collect();

	let cgroups: Vec<String> = proc
		.cgroups()
		.map(|cg| cg.0.iter().map(|c| c.pathname.clone()).collect())
		.unwrap_or_default();

	let fds: Vec<String> = if let Ok(fd_iter) = proc.fd() {
		fd_iter
			.filter_map(|fd_info| {
				fd_info.ok().map(|info| {
					use procfs::process::FDTarget;
					match info.target {
						FDTarget::Path(ref p) => {
							p.to_string_lossy().into_owned()
						}
						FDTarget::Socket(inode) => {
							format!("socket:[{}]", inode)
						}
						FDTarget::Net(inode) => {
							format!("net:[{}]", inode)
						}
						FDTarget::Pipe(inode) => {
							format!("pipe:[{}]", inode)
						}
						FDTarget::AnonInode(ref s) => {
							format!("anon_inode:[{}]", s)
						}
						FDTarget::MemFD(ref s) => {
							format!("memfd:{}", s)
						}
						FDTarget::Other(ref s, inode) => {
							format!("{}:[{}]", s, inode)
						}
					}
				})
			})
			.collect()
	} else {
		Vec::new()
	};

	let io = proc.io().ok();
	let limits_data = proc.limits().ok();

	let cpu_ticks =
		stat.utime + stat.stime + stat.cutime as u64 + stat.cstime as u64;

	let mut limits = Vec::new();
	if let Some(ref l) = limits_data {
		limits.push(LimitEntry {
			name: "CPU Time".into(),
			soft: limit_to_str(&l.max_cpu_time.soft_limit),
			hard: limit_to_str(&l.max_cpu_time.hard_limit),
		});
		limits.push(LimitEntry {
			name: "File Size".into(),
			soft: limit_to_str(&l.max_file_size.soft_limit),
			hard: limit_to_str(&l.max_file_size.hard_limit),
		});
		limits.push(LimitEntry {
			name: "Data Size".into(),
			soft: limit_to_str(&l.max_data_size.soft_limit),
			hard: limit_to_str(&l.max_data_size.hard_limit),
		});
		limits.push(LimitEntry {
			name: "Stack Size".into(),
			soft: limit_to_str(&l.max_stack_size.soft_limit),
			hard: limit_to_str(&l.max_stack_size.hard_limit),
		});
		limits.push(LimitEntry {
			name: "Core File Size".into(),
			soft: limit_to_str(&l.max_core_file_size.soft_limit),
			hard: limit_to_str(&l.max_core_file_size.hard_limit),
		});
		limits.push(LimitEntry {
			name: "Resident Set".into(),
			soft: limit_to_str(&l.max_resident_set.soft_limit),
			hard: limit_to_str(&l.max_resident_set.hard_limit),
		});
		limits.push(LimitEntry {
			name: "Processes".into(),
			soft: limit_to_str(&l.max_processes.soft_limit),
			hard: limit_to_str(&l.max_processes.hard_limit),
		});
		limits.push(LimitEntry {
			name: "Open Files".into(),
			soft: limit_to_str(&l.max_open_files.soft_limit),
			hard: limit_to_str(&l.max_open_files.hard_limit),
		});
		limits.push(LimitEntry {
			name: "Locked Memory".into(),
			soft: limit_to_str(&l.max_locked_memory.soft_limit),
			hard: limit_to_str(&l.max_locked_memory.hard_limit),
		});
		limits.push(LimitEntry {
			name: "Address Space".into(),
			soft: limit_to_str(&l.max_address_space.soft_limit),
			hard: limit_to_str(&l.max_address_space.hard_limit),
		});
		limits.push(LimitEntry {
			name: "File Locks".into(),
			soft: limit_to_str(&l.max_file_locks.soft_limit),
			hard: limit_to_str(&l.max_file_locks.hard_limit),
		});
		limits.push(LimitEntry {
			name: "Pending Signals".into(),
			soft: limit_to_str(&l.max_pending_signals.soft_limit),
			hard: limit_to_str(&l.max_pending_signals.hard_limit),
		});
		limits.push(LimitEntry {
			name: "Msgqueue Size".into(),
			soft: limit_to_str(&l.max_msgqueue_size.soft_limit),
			hard: limit_to_str(&l.max_msgqueue_size.hard_limit),
		});
		limits.push(LimitEntry {
			name: "Nice Priority".into(),
			soft: limit_to_str(&l.max_nice_priority.soft_limit),
			hard: limit_to_str(&l.max_nice_priority.hard_limit),
		});
		limits.push(LimitEntry {
			name: "Realtime Priority".into(),
			soft: limit_to_str(&l.max_realtime_priority.soft_limit),
			hard: limit_to_str(&l.max_realtime_priority.hard_limit),
		});
		limits.push(LimitEntry {
			name: "Realtime Timeout".into(),
			soft: limit_to_str(&l.max_realtime_timeout.soft_limit),
			hard: limit_to_str(&l.max_realtime_timeout.hard_limit),
		});
	}

	let smaps = proc.smaps_rollup().ok().and_then(|sr| {
		sr.memory_map_rollup.0.into_iter().next().map(|entry| {
			let map = &entry.extension.map;
			let private_clean = map.get("Private_Clean").copied();
			let private_dirty = map.get("Private_Dirty").copied();
			let uss = match (private_clean, private_dirty) {
				(Some(c), Some(d)) => Some(c + d),
				_ => None,
			};
			SmapsSummary {
				pss: map.get("Pss").copied(),
				uss,
				swap: map.get("Swap").copied(),
				shared_clean: map.get("Shared_Clean").copied(),
				shared_dirty: map.get("Shared_Dirty").copied(),
				private_clean,
				private_dirty,
				referenced: map.get("Referenced").copied(),
				anonymous: map.get("Anonymous").copied(),
			}
		})
	});

	Some(ProcessProperties {
		pid: stat.pid,
		ppid: stat.ppid,
		name: stat.comm.clone(),
		command,
		state: stat.state,
		state_label: model::state_label(stat.state).to_string(),
		threads: stat.num_threads,
		user: username,
		uid,
		groups: status.groups,
		priority: stat.priority,
		nice: stat.nice,
		cpu_time_ticks: cpu_ticks,
		cpu_time_secs: cpu_ticks as f64 / hertz,
		starttime_ticks: stat.starttime,
		processor: stat.processor,
		vm_size: status.vmsize.map(|v| v * 1024),
		vm_peak: status.vmpeak.map(|v| v * 1024),
		vm_rss: status.vmrss.map(|v| v * 1024),
		vm_hwm: status.vmhwm.map(|v| v * 1024),
		vm_data: status.vmdata.map(|v| v * 1024),
		vm_stk: status.vmstk.map(|v| v * 1024),
		vm_exe: status.vmexe.map(|v| v * 1024),
		vm_lib: status.vmlib.map(|v| v * 1024),
		vm_swap: status.vmswap.map(|v| v * 1024),
		vm_lck: status.vmlck.map(|v| v * 1024),
		vm_pin: status.vmpin.map(|v| v * 1024),
		rss_anon: status.rssanon.map(|v| v * 1024),
		rss_file: status.rssfile.map(|v| v * 1024),
		rss_shmem: status.rssshmem.map(|v| v * 1024),
		io_read_bytes: io.as_ref().map(|i| i.read_bytes),
		io_write_bytes: io.as_ref().map(|i| i.write_bytes),
		io_cancelled_write_bytes: io
			.as_ref()
			.map(|i| i.cancelled_write_bytes),
		io_read_chars: io.as_ref().map(|i| i.rchar),
		io_write_chars: io.as_ref().map(|i| i.wchar),
		io_read_syscalls: io.as_ref().map(|i| i.syscr),
		io_write_syscalls: io.as_ref().map(|i| i.syscw),
		limits,
		exe_path,
		cwd,
		root_path,
		cgroups,
		environ,
		fds,
		voluntary_ctxt_switches: status.voluntary_ctxt_switches,
		nonvoluntary_ctxt_switches: status.nonvoluntary_ctxt_switches,
		smaps,
	})
	}
}
