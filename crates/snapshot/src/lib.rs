use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use gpuitop_core::model::GpuBackend;
use gpuitop_icons::DesktopEntryCache;

pub struct PrevCpu {
	pub totals: Vec<(u64, u64)>,
}

pub struct PrevProc {
	pub utime: u64,
	pub stime: u64,
	pub cutime: u64,
	pub cstime: u64,
}

pub struct PrevDisk {
	pub read_bytes: u64,
	pub write_bytes: u64,
}

pub struct PrevNet {
	pub rx_bytes: u64,
	pub tx_bytes: u64,
}

pub struct CollectorState {
	pub prev_cpu: Option<PrevCpu>,
	pub prev_proc: HashMap<i32, PrevProc>,
	pub prev_disk: HashMap<String, PrevDisk>,
	pub prev_net: HashMap<String, PrevNet>,
	pub prev_time: Option<Instant>,
	pub current_uid: u32,
	pub user_cache: HashMap<u32, String>,
	pub desktop_cache: Arc<DesktopEntryCache>,
	pub gpu_backends: Vec<GpuBackend>,
	pub core_history: HashMap<usize, Vec<f32>>,
}

#[cfg(unix)]
fn initial_uid() -> u32 {
	unsafe { libc::getuid() }
}

#[cfg(not(unix))]
fn initial_uid() -> u32 {
	0
}

impl CollectorState {
	pub fn new(
		gpu_backends: Vec<GpuBackend>,
		desktop_cache: Arc<DesktopEntryCache>,
	) -> Self {
		Self {
			prev_cpu: None,
			prev_proc: HashMap::new(),
			prev_disk: HashMap::new(),
			prev_net: HashMap::new(),
			prev_time: None,
			current_uid: initial_uid(),
			user_cache: HashMap::new(),
			desktop_cache,
			gpu_backends,
			core_history: HashMap::new(),
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
pub use linux::{collect_snapshot, pid_alive};
#[cfg(target_os = "macos")]
pub use macos::{collect_snapshot, pid_alive};
#[cfg(target_os = "windows")]
pub use windows::{collect_snapshot, pid_alive};
