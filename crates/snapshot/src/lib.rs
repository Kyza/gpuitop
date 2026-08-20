use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
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
	// GUI status is derived from the process environment (DISPLAY /
	// WAYLAND_DISPLAY), which is fixed at exec time. Cache it per pid so we
	// only read /proc/PID/environ for newly-seen or changed processes.
	pub is_gui: bool,
	// starttime distinguishes a reused PID from the same process, so the
	// is_gui cache is never applied to a different process reusing a PID.
	pub starttime: u64,
}

pub struct PrevDisk {
	pub read_bytes: u64,
	pub write_bytes: u64,
}

pub struct PrevNet {
	pub rx_bytes: u64,
	pub tx_bytes: u64,
}

// Why a per-PID /proc read failed. `Dead` means the process exited or never
// existed; `Unavailable` means it exists but the data can't be read (e.g.
// permissions). Consumers render the two differently: dead stops polling,
// unavailable keeps polling and shows a degraded warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadError {
	Dead,
	Unavailable,
}

pub struct CollectorState {
	prev_cpu: Option<PrevCpu>,
	prev_proc: HashMap<i32, PrevProc>,
	prev_disk: HashMap<String, PrevDisk>,
	prev_net: HashMap<String, PrevNet>,
	prev_time: Option<Instant>,
	current_uid: u32,
	user_cache: HashMap<u32, String>,
	desktop_cache: Arc<DesktopEntryCache>,
	gpu_backends: Vec<GpuBackend>,
	// Shared with the settings tab: the live GpuData setting and a
	// one-shot re-detect request (set by the settings button, cleared here).
	gpu_data_enabled: Arc<AtomicBool>,
	redetect: Arc<AtomicBool>,
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
		gpu_data_enabled: Arc<AtomicBool>,
		redetect: Arc<AtomicBool>,
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
			gpu_data_enabled,
			redetect,
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
pub use linux::{collect_snapshot, username_for_uid, ProcBasics};
#[cfg(target_os = "macos")]
pub use macos::collect_snapshot;
#[cfg(target_os = "windows")]
pub use windows::collect_snapshot;
