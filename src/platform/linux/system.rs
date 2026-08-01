use std::collections::HashSet;
use std::fmt;
use std::path::Path;

use crate::model::ProcessInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitSystem {
	Systemd,
	OpenRc,
	Runit,
	Dinit,
	SysV,
	Unknown,
}

impl fmt::Display for InitSystem {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Systemd => write!(f, "systemd"),
			Self::OpenRc => write!(f, "OpenRC"),
			Self::Runit => write!(f, "runit"),
			Self::Dinit => write!(f, "dinit"),
			Self::SysV => write!(f, "SysV init"),
			Self::Unknown => write!(f, "unknown"),
		}
	}
}

pub fn detect_init() -> InitSystem {
	if Path::new("/run/systemd/system").exists() {
		return InitSystem::Systemd;
	}
	if Path::new("/run/dinit/control").exists() {
		return InitSystem::Dinit;
	}
	if Path::new("/run/runit").exists() {
		return InitSystem::Runit;
	}
	if Path::new("/run/openrc").exists() {
		return InitSystem::OpenRc;
	}
	if let Ok(exe) = std::fs::read_link("/proc/1/exe") {
		let s = exe.to_string_lossy();
		if s.contains("systemd") {
			return InitSystem::Systemd;
		}
		if s.contains("dinit") {
			return InitSystem::Dinit;
		}
		if s.contains("runit") {
			return InitSystem::Runit;
		}
		if s.contains("openrc-init") || s.contains("/sbin/init") {
			if Path::new("/run/openrc").exists() {
				return InitSystem::OpenRc;
			}
			if Path::new("/etc/inittab").exists() {
				return InitSystem::SysV;
			}
		}
	}
	if Path::new("/etc/inittab").exists() {
		return InitSystem::SysV;
	}
	InitSystem::Unknown
}

pub fn pids_of_runsv(processes: &[ProcessInfo]) -> HashSet<i32> {
	processes
		.iter()
		.filter(|p| p.name == "runsv")
		.map(|p| p.pid)
		.collect()
}

fn pids_of_supervise_daemon(processes: &[ProcessInfo]) -> HashSet<i32> {
	processes
		.iter()
		.filter(|p| p.name == "supervise-daemon")
		.map(|p| p.pid)
		.collect()
}

pub fn is_service(
	proc: &ProcessInfo,
	init: InitSystem,
	runsv_pids: &HashSet<i32>,
	supervise_pids: &HashSet<i32>,
) -> bool {
	match init {
		InitSystem::Systemd => {
			if proc.cgroup.is_empty() {
				return proc.ppid == 1;
			}
			!proc.cgroup.contains("/user.slice/")
				&& proc.cgroup.contains(".service")
		}
		InitSystem::OpenRc => {
			proc.ppid == 1
				|| supervise_pids.contains(&proc.ppid)
		}
		InitSystem::Runit => runsv_pids.contains(&proc.ppid),
		InitSystem::Dinit | InitSystem::SysV | InitSystem::Unknown => {
			proc.ppid == 1
		}
	}
}
