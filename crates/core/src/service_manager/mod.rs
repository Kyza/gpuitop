use std::collections::HashSet;
use std::fmt;

use oswap::{define_interface, define_platforms};

use crate::model::ProcessSnapshot;

#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Default,
	serde::Serialize,
	serde::Deserialize,
)]
pub enum InitSystem {
	#[default]
	Unknown,
	Systemd,
	OpenRc,
	Runit,
	Dinit,
	SysV,
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

define_interface! { Platform, ServiceManagerInterface, impl_interface,
	pub fn detect_init() -> InitSystem;
	pub fn pids_of_runsv(processes: &[ProcessSnapshot]) -> HashSet<i32>;
	pub(crate) fn pids_of_supervise_daemon(processes: &[ProcessSnapshot]) -> HashSet<i32>;
	pub fn is_service(proc: &ProcessSnapshot, init: InitSystem, runsv_pids: &HashSet<i32>, supervise_pids: &HashSet<i32>) -> bool;
}

define_platforms![
	{ file: "linux", cfg: target_os = "linux" },
	{ file: "macos", cfg: target_os = "macos" },
	{ file: "windows", cfg: target_os = "windows" },
];
