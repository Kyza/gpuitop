#[derive(Debug, Clone)]
pub struct ProcessSnapshot {
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
	pub vram: VramUsage,
	pub disk_read_bytes_per_sec: f64,
	pub disk_write_bytes_per_sec: f64,
	pub is_gui: bool,
	pub is_kthread: bool,
	pub is_owned_by_current_user: bool,
	pub is_electron: bool,
	pub electron_app_name: Option<String>,
	pub icon_name: Option<String>,
	pub has_children: bool,
}

#[derive(Debug, Clone)]
pub struct CpuCore {
	pub index: usize,
	pub usage_percent: f32,
	pub frequency_mhz: u32,
}

#[derive(Debug, Clone)]
pub struct CpuInfo {
	pub cores: Vec<CpuCore>,
	pub overall_percent: f32,
	pub model_name: String,
	pub temperature: f32,
}

#[derive(Debug, Clone)]
pub struct MemoryInfo {
	pub total: u64,
	pub used: u64,
	pub available: u64,
	pub cached: u64,
	pub swap_used: u64,
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
				model_name: String::new(),
				temperature: 0.0,
			},
			memory: MemoryInfo {
				total: 0,
				used: 0,
				available: 0,
				cached: 0,
				swap_used: 0,
			},
			disks: Vec::new(),
			networks: Vec::new(),
			timestamp: std::time::Instant::now(),
			gpu_backends: Vec::new(),
			gpu_devices: Vec::new(),
			gpu_polling_enabled: false,
		}
	}
}

#[derive(Debug, Clone)]
pub struct SystemSnapshot {
	pub processes: Vec<ProcessSnapshot>,
	pub cpu: CpuInfo,
	pub memory: MemoryInfo,
	pub disks: Vec<DiskInfo>,
	pub networks: Vec<NetInfo>,
	pub timestamp: std::time::Instant,
	pub gpu_backends: Vec<GpuBackend>,
	pub gpu_devices: Vec<GpuDevice>,
	pub gpu_polling_enabled: bool,
}

#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	serde::Serialize,
	serde::Deserialize,
	Default,
)]
pub enum ResourceViewMode {
	#[default]
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
pub enum DefaultViewMode {
	#[default]
	List,
	Tree,
}

impl std::fmt::Display for DefaultViewMode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::List => write!(f, "List"),
			Self::Tree => write!(f, "Tree"),
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
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	serde::Serialize,
	serde::Deserialize,
	Default,
)]
pub enum SortColumn {
	#[default]
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
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	serde::Serialize,
	serde::Deserialize,
	Default,
)]
pub enum PidFilterMode {
	AllDescendants,
	#[default]
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

#[derive(Debug, Clone)]
pub struct GpuDevice {
	pub backend: GpuBackend,
	pub name: String,
	pub utilization: u32,
	pub temperature: u32,
	pub vram_total: u64,
	pub vram_used: u64,
	pub power_watts: f32,
	pub core_clock_mhz: u32,
	pub memory_clock_mhz: u32,
	pub fan_percent: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VramUsage {
	pub nvidia: u64,
	pub amd: u64,
}

impl VramUsage {
	pub fn total(&self) -> u64 {
		self.nvidia + self.amd
	}

	pub fn is_empty(&self) -> bool {
		self.nvidia == 0 && self.amd == 0
	}
}

#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	serde::Serialize,
	serde::Deserialize,
	Default,
)]
pub enum GpuData {
	#[default]
	On,
	Off,
}

impl std::fmt::Display for GpuData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
	Ascending,
	Descending,
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
	Nvidia,
	Amd,
	Electron,
	ProcessState(char),
	Username(String),
	Pid(i32),
}

pub fn state_label(c: char) -> &'static str {
	match c {
		'R' => "Running",
		'S' => "Sleeping",
		'D' => "Disk Sleep",
		'Z' => "Zombie",
		'T' => "Stopped",
		't' => "Tracing Stop",
		'I' => "Idle",
		'X' => "Dead",
		_ => "Other",
	}
}

impl Filter {
	pub fn label(&self, processes: &[ProcessSnapshot]) -> String {
		match self {
			Self::Gui => "GUI".into(),
			Self::User => "User".into(),
			Self::System => "System".into(),
			Self::Services => "Services".into(),
			Self::Kernel => "Kernel".into(),
			Self::Parent => "Parent".into(),
			Self::Vram => "VRAM".into(),
			Self::Nvidia => "NVIDIA".into(),
			Self::Amd => "AMD".into(),
			Self::Electron => "Electron".into(),
			Self::ProcessState(c) => state_label(*c).into(),
			Self::Username(s) => s.clone(),
			Self::Pid(pid) => processes
				.iter()
				.find(|p| p.pid == *pid)
				.map(|p| p.name.clone())
				.unwrap_or_else(|| pid.to_string()),
		}
	}
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

#[cfg(test)]
mod tests {
	use super::*;

	fn make_proc(pid: i32, ppid: i32) -> ProcessSnapshot {
		ProcessSnapshot {
			pid,
			ppid,
			name: "test".into(),
			user: "root".into(),
			state: 'S',
			command: "test".into(),
			cgroup: String::new(),
			cpu_percent: 0.0,
			mem_percent: 0.0,
			mem_rss: 0,
			vram: VramUsage::default(),
			disk_read_bytes_per_sec: 0.0,
			disk_write_bytes_per_sec: 0.0,
			is_gui: false,
			is_kthread: false,
			is_owned_by_current_user: false,
			is_electron: false,
			electron_app_name: None,
			icon_name: None,
			has_children: false,
		}
	}

	#[test]
	fn sort_column_all_returns_9_variants() {
		let cols = SortColumn::all();
		assert_eq!(cols.len(), 9);
		assert!(cols.contains(&SortColumn::Name));
		assert!(cols.contains(&SortColumn::Pid));
		assert!(cols.contains(&SortColumn::User));
		assert!(cols.contains(&SortColumn::State));
		assert!(cols.contains(&SortColumn::Cpu));
		assert!(cols.contains(&SortColumn::Memory));
		assert!(cols.contains(&SortColumn::Vram));
		assert!(cols.contains(&SortColumn::DiskRead));
		assert!(cols.contains(&SortColumn::DiskWrite));
	}

	#[test]
	fn sort_column_to_col_index() {
		assert_eq!(SortColumn::Name.to_col_index(), 1);
		assert_eq!(SortColumn::Pid.to_col_index(), 2);
		assert_eq!(SortColumn::User.to_col_index(), 3);
		assert_eq!(SortColumn::State.to_col_index(), 4);
		assert_eq!(SortColumn::Cpu.to_col_index(), 5);
		assert_eq!(SortColumn::Memory.to_col_index(), 6);
		assert_eq!(SortColumn::Vram.to_col_index(), 7);
		assert_eq!(SortColumn::DiskRead.to_col_index(), 8);
		assert_eq!(SortColumn::DiskWrite.to_col_index(), 9);
	}

	#[test]
	fn sort_column_from_col_index_roundtrip() {
		for col in SortColumn::all() {
			let ix = col.to_col_index();
			let back = SortColumn::from_col_index(ix);
			assert_eq!(back, Some(col), "roundtrip failed for {:?}", col);
		}
	}

	#[test]
	fn sort_column_from_col_index_valid_indices() {
		assert_eq!(SortColumn::from_col_index(1), Some(SortColumn::Name));
		assert_eq!(SortColumn::from_col_index(2), Some(SortColumn::Pid));
		assert_eq!(SortColumn::from_col_index(3), Some(SortColumn::User));
		assert_eq!(SortColumn::from_col_index(4), Some(SortColumn::State));
		assert_eq!(SortColumn::from_col_index(5), Some(SortColumn::Cpu));
		assert_eq!(SortColumn::from_col_index(6), Some(SortColumn::Memory));
		assert_eq!(SortColumn::from_col_index(7), Some(SortColumn::Vram));
		assert_eq!(SortColumn::from_col_index(8), Some(SortColumn::DiskRead));
		assert_eq!(
			SortColumn::from_col_index(9),
			Some(SortColumn::DiskWrite)
		);
	}

	#[test]
	fn sort_column_from_col_index_invalid() {
		assert_eq!(SortColumn::from_col_index(0), None);
		assert_eq!(SortColumn::from_col_index(10), None);
		assert_eq!(SortColumn::from_col_index(999), None);
	}

	#[test]
	fn state_label_all_branches() {
		assert_eq!(state_label('R'), "Running");
		assert_eq!(state_label('S'), "Sleeping");
		assert_eq!(state_label('D'), "Disk Sleep");
		assert_eq!(state_label('Z'), "Zombie");
		assert_eq!(state_label('T'), "Stopped");
		assert_eq!(state_label('t'), "Tracing Stop");
		assert_eq!(state_label('I'), "Idle");
		assert_eq!(state_label('X'), "Dead");
		assert_eq!(state_label('?'), "Other");
		assert_eq!(state_label('A'), "Other");
		assert_eq!(state_label('z'), "Other");
	}

	#[test]
	fn filter_label_simple_variants() {
		let procs = vec![];
		assert_eq!(Filter::Gui.label(&procs), "GUI");
		assert_eq!(Filter::User.label(&procs), "User");
		assert_eq!(Filter::System.label(&procs), "System");
		assert_eq!(Filter::Services.label(&procs), "Services");
		assert_eq!(Filter::Kernel.label(&procs), "Kernel");
		assert_eq!(Filter::Parent.label(&procs), "Parent");
		assert_eq!(Filter::Vram.label(&procs), "VRAM");
		assert_eq!(Filter::Nvidia.label(&procs), "NVIDIA");
		assert_eq!(Filter::Amd.label(&procs), "AMD");
		assert_eq!(Filter::Electron.label(&procs), "Electron");
	}

	#[test]
	fn filter_label_process_state() {
		let procs = vec![];
		assert_eq!(Filter::ProcessState('R').label(&procs), "Running");
		assert_eq!(Filter::ProcessState('S').label(&procs), "Sleeping");
		assert_eq!(Filter::ProcessState('X').label(&procs), "Dead");
		assert_eq!(Filter::ProcessState('?').label(&procs), "Other");
	}

	#[test]
	fn filter_label_username() {
		let procs = vec![];
		assert_eq!(
			Filter::Username("testuser".into()).label(&procs),
			"testuser"
		);
		assert_eq!(Filter::Username("root".into()).label(&procs), "root");
	}

	#[test]
	fn filter_label_pid_returns_name() {
		let procs = vec![ProcessSnapshot {
			pid: 42,
			ppid: 1,
			name: "parentish".into(),
			..make_proc(42, 1)
		}];
		let label = Filter::Pid(42).label(&procs);
		assert_eq!(label, "parentish");
	}

	#[test]
	fn filter_label_pid_missing_uses_pid_string() {
		let procs = vec![];
		let label = Filter::Pid(12345).label(&procs);
		assert_eq!(label, "12345");
	}

	#[test]
	fn display_resource_view_mode() {
		assert_eq!(ResourceViewMode::SelfOnly.to_string(), "Self");
		assert_eq!(ResourceViewMode::Cumulative.to_string(), "Cumulative");
	}

	#[test]
	fn display_default_view_mode() {
		assert_eq!(DefaultViewMode::List.to_string(), "List");
		assert_eq!(DefaultViewMode::Tree.to_string(), "Tree");
	}

	#[test]
	fn display_process_grouping() {
		assert_eq!(ProcessGrouping::Auto.to_string(), "Auto");
		assert_eq!(ProcessGrouping::ByUser.to_string(), "By User");
		assert_eq!(ProcessGrouping::ByState.to_string(), "By State");
		assert_eq!(ProcessGrouping::Flat.to_string(), "Flat");
	}

	#[test]
	fn display_pid_filter_mode() {
		assert_eq!(
			PidFilterMode::AllDescendants.to_string(),
			"All descendants"
		);
		assert_eq!(PidFilterMode::DirectChildren.to_string(), "Direct only");
	}

	#[test]
	fn display_gpu_data() {
		assert_eq!(GpuData::On.to_string(), "On");
		assert_eq!(GpuData::Off.to_string(), "Off");
	}

	#[test]
	fn display_gpu_backend() {
		assert_eq!(GpuBackend::None.to_string(), "None");
		assert_eq!(GpuBackend::Nvidia.to_string(), "NVIDIA");
		assert_eq!(GpuBackend::Amd.to_string(), "AMD");
	}

	#[test]
	fn vram_usage_default_empty() {
		let v = VramUsage::default();
		assert_eq!(v.total(), 0);
		assert!(v.is_empty());
	}

	#[test]
	fn vram_usage_total_sums_vendors() {
		let v = VramUsage {
			nvidia: 2048,
			amd: 1024,
		};
		assert_eq!(v.total(), 3072);
		assert!(!v.is_empty());
	}

	#[test]
	fn vram_usage_single_vendor_not_empty() {
		let v = VramUsage { nvidia: 1, amd: 0 };
		assert_eq!(v.total(), 1);
		assert!(!v.is_empty());
	}

	#[test]
	fn display_filter_mode() {
		assert_eq!(FilterMode::And.to_string(), "AND");
		assert_eq!(FilterMode::Or.to_string(), "OR");
	}
}
