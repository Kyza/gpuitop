use crate::data::icons::DesktopEntryCache;
use crate::data::model::GpuBackend;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

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
	pub gpu_backend: GpuBackend,
	pub core_history: HashMap<usize, Vec<f32>>,
}

impl CollectorState {
	pub fn new(
		gpu_backend: GpuBackend,
		desktop_cache: Arc<DesktopEntryCache>,
	) -> Self {
		Self {
			prev_cpu: None,
			prev_proc: HashMap::new(),
			prev_disk: HashMap::new(),
			prev_net: HashMap::new(),
			prev_time: None,
			current_uid: unsafe { libc::getuid() },
			user_cache: HashMap::new(),
			desktop_cache,
			gpu_backend,
			core_history: HashMap::new(),
		}
	}
}
