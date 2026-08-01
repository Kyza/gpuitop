#[derive(Debug, Clone)]
pub struct ProcessInfo {
	pub pid: i32,
	pub ppid: i32,
	pub name: String,
	pub user: String,
	pub state: char,
	pub command: String,
	pub cgroup: String,
	pub cpu_percent: f32,
	pub mem_percent: f32,
	pub mem_rss: u64,
	pub vram_bytes: Option<u64>,
	pub disk_read_bytes_per_sec: f64,
	pub disk_write_bytes_per_sec: f64,
	pub is_gui: bool,
	pub is_kthread: bool,
	pub is_owned_by_current_user: bool,
	pub is_electron: bool,
	pub electron_app_name: Option<String>,
	pub children: Vec<ProcessInfo>,
	pub has_children: bool,
}

#[derive(Debug, Clone)]
pub struct CpuCore {
	pub index: usize,
	pub usage_percent: f32,
	pub history: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct CpuInfo {
	pub cores: Vec<CpuCore>,
	pub overall_percent: f32,
}

#[derive(Debug, Clone)]
pub struct MemoryInfo {
	pub total: u64,
	pub used: u64,
	pub available: u64,
	pub free: u64,
	pub buffers: u64,
	pub cached: u64,
	pub swap_total: u64,
	pub swap_used: u64,
	pub swap_free: u64,
}

#[derive(Debug, Clone)]
pub struct DiskInfo {
	pub device: String,
	pub read_bytes_per_sec: f64,
	pub write_bytes_per_sec: f64,
}

#[derive(Debug, Clone)]
pub struct NetInfo {
	pub interface: String,
	pub rx_bytes_per_sec: f64,
	pub tx_bytes_per_sec: f64,
}

impl SystemSnapshot {
	pub fn empty() -> Self {
		Self {
			processes: Vec::new(),
			cpu: CpuInfo {
				cores: Vec::new(),
				overall_percent: 0.0,
			},
			memory: MemoryInfo {
				total: 0,
				used: 0,
				available: 0,
				free: 0,
				buffers: 0,
				cached: 0,
				swap_total: 0,
				swap_used: 0,
				swap_free: 0,
			},
			disks: Vec::new(),
			networks: Vec::new(),
			timestamp: std::time::Instant::now(),
			gpu_backend: GpuBackend::None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct SystemSnapshot {
	pub processes: Vec<ProcessInfo>,
	pub cpu: CpuInfo,
	pub memory: MemoryInfo,
	pub disks: Vec<DiskInfo>,
	pub networks: Vec<NetInfo>,
	pub timestamp: std::time::Instant,
	pub gpu_backend: GpuBackend,
}

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub enum ResourceViewMode {
	SelfOnly,
	Cumulative,
}

impl std::fmt::Display for ResourceViewMode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::SelfOnly => write!(f, "Self"),
			Self::Cumulative => write!(f, "Cumulative"),
		}
	}
}

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
pub enum ProcessGrouping {
	#[default]
	Auto,
	ByUser,
	ByState,
	Flat,
}

impl std::fmt::Display for ProcessGrouping {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Auto => write!(f, "Auto"),
			Self::ByUser => write!(f, "By User"),
			Self::ByState => write!(f, "By State"),
			Self::Flat => write!(f, "Flat"),
		}
	}
}

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub enum SortColumn {
	Name,
	Pid,
	User,
	State,
	Cpu,
	Memory,
	Vram,
	DiskRead,
	DiskWrite,
}

impl SortColumn {
	pub fn all() -> Vec<SortColumn> {
		vec![
			Self::Name,
			Self::Pid,
			Self::User,
			Self::State,
			Self::Cpu,
			Self::Memory,
			Self::Vram,
			Self::DiskRead,
			Self::DiskWrite,
		]
	}

	pub fn to_col_index(self) -> usize {
		match self {
			Self::Name => 1,
			Self::Pid => 2,
			Self::User => 3,
			Self::State => 4,
			Self::Cpu => 5,
			Self::Memory => 6,
			Self::Vram => 7,
			Self::DiskRead => 8,
			Self::DiskWrite => 9,
		}
	}

	pub fn from_col_index(ix: usize) -> Option<Self> {
		match ix {
			1 => Some(Self::Name),
			2 => Some(Self::Pid),
			3 => Some(Self::User),
			4 => Some(Self::State),
			5 => Some(Self::Cpu),
			6 => Some(Self::Memory),
			7 => Some(Self::Vram),
			8 => Some(Self::DiskRead),
			9 => Some(Self::DiskWrite),
			_ => None,
		}
	}
}

impl std::fmt::Display for SortColumn {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Name => write!(f, "Name"),
			Self::Pid => write!(f, "PID"),
			Self::User => write!(f, "User"),
			Self::State => write!(f, "State"),
			Self::Cpu => write!(f, "CPU%"),
			Self::Memory => write!(f, "Memory%"),
			Self::Vram => write!(f, "VRAM"),
			Self::DiskRead => write!(f, "Disk R"),
			Self::DiskWrite => write!(f, "Disk W"),
		}
	}
}

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub enum PidFilterMode {
	AllDescendants,
	DirectChildren,
}

impl std::fmt::Display for PidFilterMode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::AllDescendants => write!(f, "All descendants"),
			Self::DirectChildren => write!(f, "Direct only"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBackend {
	None,
	Nvidia,
	Amd,
}

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub enum Theme {
	Dark,
	Light,
	System,
}

impl std::fmt::Display for Theme {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Dark => write!(f, "Dark"),
			Self::Light => write!(f, "Light"),
			Self::System => write!(f, "System"),
		}
	}
}

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub enum VramPolling {
	Auto,
	On,
	Off,
}

impl std::fmt::Display for VramPolling {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Auto => write!(f, "Auto"),
			Self::On => write!(f, "On"),
			Self::Off => write!(f, "Off"),
		}
	}
}

impl std::fmt::Display for GpuBackend {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::None => write!(f, "None"),
			Self::Nvidia => write!(f, "NVIDIA"),
			Self::Amd => write!(f, "AMD"),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Filter {
	Gui,
	User,
	System,
	Services,
	Kernel,
	Parent,
	Vram,
	Electron,
	ProcessState(char),
	Pid(i32),
}

/// Human-readable label for a Linux process state character.
pub fn state_label(c: char) -> &'static str {
	match c {
		'R' => "Running",
		'S' => "Sleeping",
		'D' => "Disk Sleep",
		'Z' => "Zombie",
		'T' => "Stopped",
		't' => "Stopped",
		'I' => "Idle",
		'X' => "Dead",
		_ => "Other",
	}
}

impl Filter {
	pub fn is_type_filter(&self) -> bool {
		matches!(
			self,
			Self::Gui
				| Self::User | Self::System
				| Self::Services
				| Self::Kernel
				| Self::Parent
				| Self::Vram | Self::Electron
				| Self::ProcessState(_)
		)
	}

	pub fn label(&self, processes: &[ProcessInfo]) -> String {
		match self {
			Self::Gui => "GUI".into(),
			Self::User => "User".into(),
			Self::System => "System".into(),
			Self::Services => "Services".into(),
			Self::Kernel => "Kernel".into(),
			Self::Parent => "Parent".into(),
			Self::Vram => "VRAM".into(),
			Self::Electron => "Electron".into(),
			Self::ProcessState(c) => state_label(*c).into(),
			Self::Pid(pid) => {
				let count = processes
					.iter()
					.filter(|p| is_descendant_of_flat(p.pid, *pid, processes))
					.count();
				let name = processes
					.iter()
					.find(|p| p.pid == *pid)
					.map(|p| p.name.clone())
					.unwrap_or_else(|| pid.to_string());
				format!("{name} (+{count} children)")
			}
		}
	}
}

fn is_descendant_of_flat(
	child_pid: i32,
	ancestor: i32,
	all: &[ProcessInfo],
) -> bool {
	let mut current = child_pid;
	for _ in 0..100 {
		if current == ancestor {
			return true;
		}
		if let Some(p) = all.iter().find(|p| p.pid == current) {
			current = p.ppid;
		} else {
			return false;
		}
	}
	false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
	And,
	Or,
}

impl std::fmt::Display for FilterMode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::And => write!(f, "AND"),
			Self::Or => write!(f, "OR"),
		}
	}
}
