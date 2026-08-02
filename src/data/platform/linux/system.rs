use std::collections::HashSet;
use std::fmt;
use std::path::Path;

use crate::data::model::ProcessInfo;

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

pub(crate) fn pids_of_supervise_daemon(
	processes: &[ProcessInfo],
) -> HashSet<i32> {
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
			proc.ppid == 1 || supervise_pids.contains(&proc.ppid)
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
		assert!(is_systemd_service_cgroup("0::/system.slice/sshd.service\n"));
	}

	#[test]
	fn systemd_cgroup_matches_user_service() {
		assert!(is_systemd_service_cgroup(
			"0::/user.slice/user-1000.slice/user@1000.service/app.slice/\
			 gnome-keyring-daemon.service\n"
		));
	}

	#[test]
	fn systemd_cgroup_rejects_scope_unit() {
		assert!(!is_systemd_service_cgroup(
			"0::/user.slice/user-1000.slice/user@1000.service/app.slice/\
			 app-ghostty-surface-transient-1562569.scope\n"
		));
	}

	#[test]
	fn systemd_cgroup_rejects_empty() {
		assert!(!is_systemd_service_cgroup(""));
	}

	#[test]
	fn systemd_cgroup_no_newline() {
		assert!(is_systemd_service_cgroup("0::/system.slice/cron.service"));
	}

	#[test]
	fn systemd_cgroup_init_scope_rejected() {
		assert!(!is_systemd_service_cgroup("0::/init.scope\n"));
	}

	#[test]
	fn detect_init_on_current_system() {
		let _init = detect_init();
	}

	fn sproc(pid: i32, ppid: i32, cgroup: &str) -> ProcessInfo {
		ProcessInfo {
			pid,
			ppid,
			name: "test".into(),
			user: "root".into(),
			state: 'S',
			command: "test".into(),
			cgroup: cgroup.into(),
			cpu_percent: 0.0,
			mem_percent: 0.0,
			mem_rss: 0,
			vram_bytes: None,
			disk_read_bytes_per_sec: 0.0,
			disk_write_bytes_per_sec: 0.0,
			is_gui: false,
			is_kthread: false,
			is_owned_by_current_user: false,
			is_electron: false,
			electron_app_name: None,
			children: vec![],
			has_children: false,
		}
	}

	fn runit_proc(pid: i32, ppid: i32) -> ProcessInfo {
		let mut p = sproc(pid, ppid, "");
		p.name = "runsv".into();
		p
	}

	fn supervise_proc(pid: i32, ppid: i32) -> ProcessInfo {
		let mut p = sproc(pid, ppid, "");
		p.name = "supervise-daemon".into();
		p
	}

	// ── pids_of_runsv ──────────────────────────────────────

	#[test]
	fn pids_of_runsv_empty() {
		assert!(pids_of_runsv(&[]).is_empty());
	}

	#[test]
	fn pids_of_runsv_single() {
		let procs = [runit_proc(42, 1)];
		let pids = pids_of_runsv(&procs);
		assert_eq!(pids.len(), 1);
		assert!(pids.contains(&42));
	}

	#[test]
	fn pids_of_runsv_multiple() {
		let procs = [runit_proc(10, 1), runit_proc(20, 1), runit_proc(30, 1)];
		let pids = pids_of_runsv(&procs);
		assert_eq!(pids.len(), 3);
		assert!(pids.contains(&10));
		assert!(pids.contains(&20));
		assert!(pids.contains(&30));
	}

	#[test]
	fn pids_of_runsv_excludes_non_runsv() {
		let procs = [
			runit_proc(10, 1),
			sproc(20, 1, ""),
			runit_proc(30, 1),
			sproc(40, 1, ""),
		];
		let pids = pids_of_runsv(&procs);
		assert_eq!(pids.len(), 2);
		assert!(pids.contains(&10));
		assert!(pids.contains(&30));
	}

	// ── pids_of_supervise_daemon ────────────────────────────

	#[test]
	fn pids_of_supervise_empty() {
		assert!(pids_of_supervise_daemon(&[]).is_empty());
	}

	#[test]
	fn pids_of_supervise_single() {
		let procs = [supervise_proc(99, 1)];
		let pids = pids_of_supervise_daemon(&procs);
		assert_eq!(pids.len(), 1);
		assert!(pids.contains(&99));
	}

	#[test]
	fn pids_of_supervise_excludes_non_matching() {
		let procs =
			[supervise_proc(10, 1), sproc(20, 1, ""), sproc(30, 1, "")];
		let pids = pids_of_supervise_daemon(&procs);
		assert_eq!(pids.len(), 1);
		assert!(pids.contains(&10));
	}

	// ── is_service ──────────────────────────────────────────

	#[test]
	fn is_service_systemd_with_service_cgroup() {
		let p = sproc(100, 2, "0::/system.slice/sshd.service\n");
		assert!(is_service(
			&p,
			InitSystem::Systemd,
			&HashSet::new(),
			&HashSet::new()
		));
	}

	#[test]
	fn is_service_systemd_empty_cgroup_ppid_1() {
		let p = sproc(100, 1, "");
		assert!(is_service(
			&p,
			InitSystem::Systemd,
			&HashSet::new(),
			&HashSet::new()
		));
	}

	#[test]
	fn is_service_systemd_empty_cgroup_ppid_not_1() {
		let p = sproc(100, 999, "");
		assert!(!is_service(
			&p,
			InitSystem::Systemd,
			&HashSet::new(),
			&HashSet::new()
		));
	}

	#[test]
	fn is_service_openrc_ppid_1() {
		let p = sproc(100, 1, "");
		assert!(is_service(
			&p,
			InitSystem::OpenRc,
			&HashSet::new(),
			&HashSet::new()
		));
	}

	#[test]
	fn is_service_openrc_ppid_in_supervise() {
		let superv: HashSet<i32> = [42].into();
		let p = sproc(100, 42, "");
		assert!(is_service(&p, InitSystem::OpenRc, &HashSet::new(), &superv));
	}

	#[test]
	fn is_service_openrc_ppid_not_1_not_supervise() {
		let p = sproc(100, 999, "");
		assert!(!is_service(
			&p,
			InitSystem::OpenRc,
			&HashSet::new(),
			&HashSet::new()
		));
	}

	#[test]
	fn is_service_runit_ppid_in_runsv() {
		let runsv: HashSet<i32> = [55].into();
		let p = sproc(100, 55, "");
		assert!(is_service(&p, InitSystem::Runit, &runsv, &HashSet::new()));
	}

	#[test]
	fn is_service_runit_ppid_not_in_runsv() {
		let p = sproc(100, 999, "");
		assert!(!is_service(
			&p,
			InitSystem::Runit,
			&HashSet::new(),
			&HashSet::new()
		));
	}

	#[test]
	fn is_service_dinit_ppid_1() {
		let p = sproc(100, 1, "");
		assert!(is_service(
			&p,
			InitSystem::Dinit,
			&HashSet::new(),
			&HashSet::new()
		));
	}

	#[test]
	fn is_service_sysv_ppid_1() {
		let p = sproc(100, 1, "");
		assert!(is_service(
			&p,
			InitSystem::SysV,
			&HashSet::new(),
			&HashSet::new()
		));
	}

	#[test]
	fn is_service_unknown_ppid_1() {
		let p = sproc(100, 1, "");
		assert!(is_service(
			&p,
			InitSystem::Unknown,
			&HashSet::new(),
			&HashSet::new()
		));
	}

	#[test]
	fn is_service_unknown_ppid_not_1() {
		let p = sproc(100, 999, "");
		assert!(!is_service(
			&p,
			InitSystem::Unknown,
			&HashSet::new(),
			&HashSet::new()
		));
	}
}
