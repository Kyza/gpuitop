use std::fmt;

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

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub(crate) use linux::pids_of_supervise_daemon;
#[cfg(target_os = "linux")]
pub use linux::{detect_init, is_service, pids_of_runsv};
#[cfg(target_os = "macos")]
pub(crate) use macos::pids_of_supervise_daemon;
#[cfg(target_os = "macos")]
pub use macos::{detect_init, is_service, pids_of_runsv};
#[cfg(target_os = "windows")]
pub(crate) use windows::pids_of_supervise_daemon;
#[cfg(target_os = "windows")]
pub use windows::{detect_init, is_service, pids_of_runsv};
