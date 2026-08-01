use std::collections::HashSet;
use std::fmt;
use std::path::Path;

use crate::model::ProcessInfo;

/// Check if a cgroup path belongs to a systemd service unit.
///
/// The cgroup string comes from `/proc/{pid}/cgroup` and may contain a
/// trailing newline. On cgroups v2 the format is `0::/path/to/unit`.
/// A service unit's cgroup segment ends with `.service`.
pub fn is_systemd_service_cgroup(cgroup: &str) -> bool {
	cgroup
		.lines()
		.next()
		.unwrap_or("")
		.trim()
		.rsplit('/')
		.next()
		.map(|s| s.ends_with(".service"))
		.unwrap_or(false)
}

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
			is_systemd_service_cgroup(&proc.cgroup)
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn systemd_cgroup_matches_service() {
		assert!(is_systemd_service_cgroup(
			"0::/system.slice/sshd.service\n"
		));
	}

	#[test]
	fn systemd_cgroup_matches_user_service() {
		assert!(is_systemd_service_cgroup(
			"0::/user.slice/user-1000.slice/user@1000.service/app.slice/gnome-keyring-daemon.service\n"
		));
	}

	#[test]
	fn systemd_cgroup_rejects_scope_unit() {
		assert!(!is_systemd_service_cgroup(
			"0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-ghostty-surface-transient-1562569.scope\n"
		));
	}

	#[test]
	fn systemd_cgroup_rejects_empty() {
		assert!(!is_systemd_service_cgroup(""));
	}

	#[test]
	fn systemd_cgroup_no_newline() {
		assert!(is_systemd_service_cgroup(
			"0::/system.slice/cron.service"
		));
	}

	#[test]
	fn systemd_cgroup_init_scope_rejected() {
		assert!(!is_systemd_service_cgroup(
			"0::/init.scope\n"
		));
	}

	#[test]
	fn detect_init_on_current_system() {
		// Just checks the function doesn't panic
		let _init = detect_init();
	}
}
