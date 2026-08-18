#[derive(Debug, Clone)]
pub struct ProcessProperties {
	pub pid: i32,
	pub ppid: i32,
	pub name: String,
	pub command: Vec<String>,
	pub state: char,
	pub state_label: String,
	pub threads: i64,
	pub user: String,
	pub uid: u32,
	pub groups: Vec<i32>,
	pub priority: i64,
	pub nice: i64,
	pub cpu_time_ticks: u64,
	pub cpu_time_secs: f64,
	pub starttime_ticks: u64,
	pub processor: Option<i32>,
	pub vm_size: Option<u64>,
	pub vm_peak: Option<u64>,
	pub vm_rss: Option<u64>,
	pub vm_hwm: Option<u64>,
	pub vm_data: Option<u64>,
	pub vm_stk: Option<u64>,
	pub vm_exe: Option<u64>,
	pub vm_lib: Option<u64>,
	pub vm_swap: Option<u64>,
	pub vm_lck: Option<u64>,
	pub vm_pin: Option<u64>,
	pub rss_anon: Option<u64>,
	pub rss_file: Option<u64>,
	pub rss_shmem: Option<u64>,
	pub io_read_bytes: Option<u64>,
	pub io_write_bytes: Option<u64>,
	pub io_cancelled_write_bytes: Option<u64>,
	pub io_read_chars: Option<u64>,
	pub io_write_chars: Option<u64>,
	pub io_read_syscalls: Option<u64>,
	pub io_write_syscalls: Option<u64>,
	pub limits: Vec<LimitEntry>,
	pub exe_path: Option<String>,
	pub cwd: Option<String>,
	pub root_path: Option<String>,
	pub cgroups: Vec<String>,
	pub environ: Vec<(String, String)>,
	pub fds: Vec<String>,
	pub voluntary_ctxt_switches: Option<u64>,
	pub nonvoluntary_ctxt_switches: Option<u64>,
	pub smaps: Option<SmapsSummary>,
}

#[derive(Debug, Clone)]
pub struct LimitEntry {
	pub name: String,
	pub soft: String,
	pub hard: String,
}

#[derive(Debug, Clone)]
pub struct SmapsSummary {
	pub pss: Option<u64>,
	pub uss: Option<u64>,
	pub swap: Option<u64>,
	pub shared_clean: Option<u64>,
	pub shared_dirty: Option<u64>,
	pub private_clean: Option<u64>,
	pub private_dirty: Option<u64>,
	pub referenced: Option<u64>,
	pub anonymous: Option<u64>,
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub mod window;

#[cfg(target_os = "linux")]
pub use linux::collect;
#[cfg(target_os = "macos")]
pub use macos::collect;
#[cfg(target_os = "windows")]
pub use windows::collect;

pub use window::PropertiesWindow;
