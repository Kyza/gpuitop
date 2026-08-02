use crate::data::model::{
	DefaultViewMode, PidFilterMode, ProcessGrouping, ResourceViewMode,
	SortColumn, Theme, VramPolling,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_true() -> bool {
	true
}

fn default_sort_column() -> SortColumn {
	SortColumn::Cpu
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceConfig {
	#[serde(default)]
	pub refresh_ms: u64,
	#[serde(default)]
	pub theme: Theme,
}

impl Default for InterfaceConfig {
	fn default() -> Self {
		Self {
			refresh_ms: 1500,
			theme: Theme::System,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneralConfig {
	#[serde(default)]
	pub interface: InterfaceConfig,
}

impl Default for GeneralConfig {
	fn default() -> Self {
		Self {
			interface: InterfaceConfig::default(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnEntry {
	#[serde(default)]
	pub column: SortColumn,
	#[serde(default = "default_true")]
	pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SortConfig {
	#[serde(default = "default_sort_column")]
	pub column: SortColumn,
	#[serde(default = "default_true")]
	pub descending: bool,
}

impl Default for SortConfig {
	fn default() -> Self {
		Self {
			column: SortColumn::Cpu,
			descending: true,
		}
	}
}

impl Default for ColumnEntry {
	fn default() -> Self {
		Self {
			column: SortColumn::Name,
			visible: true,
		}
	}
}

fn default_column_layout() -> Vec<ColumnEntry> {
	SortColumn::all()
		.into_iter()
		.map(|c| ColumnEntry {
			column: c,
			visible: true,
		})
		.collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviourConfig {
	#[serde(default)]
	pub vram_polling: VramPolling,
	#[serde(default)]
	pub pid_filter_mode: PidFilterMode,
	#[serde(default = "default_true")]
	pub clear_search_on_pin: bool,
	#[serde(default)]
	pub resource_view_mode: ResourceViewMode,
	#[serde(default)]
	pub default_view_mode: DefaultViewMode,
}

impl Default for BehaviourConfig {
	fn default() -> Self {
		Self {
			vram_polling: VramPolling::Auto,
			pid_filter_mode: PidFilterMode::DirectChildren,
			clear_search_on_pin: true,
			resource_view_mode: ResourceViewMode::SelfOnly,
			default_view_mode: DefaultViewMode::List,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessesConfig {
	#[serde(default)]
	pub behaviour: BehaviourConfig,
	#[serde(default = "default_column_layout")]
	pub columns: Vec<ColumnEntry>,
	#[serde(default)]
	pub default_sort: SortConfig,
}

impl Default for ProcessesConfig {
	fn default() -> Self {
		Self {
			behaviour: BehaviourConfig::default(),
			columns: default_column_layout(),
			default_sort: SortConfig::default(),
		}
	}
}

impl ProcessesConfig {
	pub fn is_col_visible(&self, col: SortColumn) -> bool {
		self.columns
			.iter()
			.find(|e| e.column == col)
			.map(|e| e.visible)
			.unwrap_or(true)
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
	#[serde(default)]
	pub general: GeneralConfig,
	#[serde(default)]
	pub processes: ProcessesConfig,
	#[serde(default)]
	pub default_grouping: ProcessGrouping,
	#[serde(default)]
	pub disk_devices: Vec<String>,
	#[serde(default)]
	pub network_interfaces: Vec<String>,
	#[serde(default)]
	pub window_size: (u32, u32),
}

impl Default for Config {
	fn default() -> Self {
		Self {
			general: GeneralConfig::default(),
			processes: ProcessesConfig::default(),
			default_grouping: ProcessGrouping::Auto,
			disk_devices: Vec::new(),
			network_interfaces: Vec::new(),
			window_size: (1100, 700),
		}
	}
}

impl Config {
	pub fn apply_override(&mut self, ron_str: &str) -> Result<()> {
		let base_ron = ron::to_string(self)
			.with_context(|| "Failed to serialize base config")?;
		let mut base_value: ron2::Value = base_ron
			.parse()
			.with_context(|| "Failed to parse base config")?;
		let overlay_value: ron2::Value = ron_str
			.parse()
			.with_context(|| "Failed to parse --override RON")?;
		crate::data::merge::deep_merge(&mut base_value, &overlay_value);
		let merged_ron = base_value.to_string();
		*self = ron::from_str(&merged_ron)
			.with_context(|| "Failed to apply override")?;
		Ok(())
	}

	pub fn load_from(path: Option<&std::path::Path>) -> Self {
		if let Some(path) = path {
			match std::fs::read_to_string(path) {
				Ok(data) => match ron::from_str(&data) {
					Ok(config) => return config,
					Err(e) => eprintln!(
						"Failed to parse config '{}': {e}. Using defaults.",
						path.display()
					),
				},
				Err(e) => eprintln!(
					"Failed to read config '{}': {e}. Using defaults.",
					path.display()
				),
			}
			return Self::default();
		}
		Self::load()
	}

	pub fn config_path() -> PathBuf {
		let base = dirs::config_dir()
			.unwrap_or_else(|| PathBuf::from("."))
			.join("gpuitop");
		base.join("config.ron")
	}

	fn ensure_dir() -> Result<PathBuf> {
		let base = dirs::config_dir()
			.unwrap_or_else(|| PathBuf::from("."))
			.join("gpuitop");
		std::fs::create_dir_all(&base).with_context(|| {
			format!("Failed to create config dir: {:?}", base)
		})?;
		Ok(base)
	}

	pub fn load() -> Self {
		let path = Self::config_path();
		if path.exists() {
			match std::fs::read_to_string(&path) {
				Ok(data) => match ron::from_str(&data) {
					Ok(config) => return config,
					Err(e) => eprintln!(
						"Failed to parse config '{}': {e}. Using defaults.",
						path.display()
					),
				},
				Err(e) => eprintln!(
					"Failed to read config '{}': {e}. Using defaults.",
					path.display()
				),
			}
		}
		let config = Self::default();
		if let Err(e) = config.save() {
			eprintln!("Failed to save default config: {e}");
		}
		config
	}

	pub fn save(&self) -> Result<()> {
		let base = Self::ensure_dir()?;
		let path = base.join("config.ron");
		let data = ron::ser::to_string_pretty(
			self,
			ron::ser::PrettyConfig::default(),
		)
		.with_context(|| "Failed to serialize config")?;
		std::fs::write(&path, data).with_context(|| {
			format!("Failed to write config to {:?}", path)
		})?;
		Ok(())
	}
}

mod dirs {
	use std::path::PathBuf;

	pub fn config_dir() -> Option<PathBuf> {
		if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
			if !dir.is_empty() {
				return Some(PathBuf::from(dir));
			}
		}
		if let Ok(home) = std::env::var("HOME") {
			return Some(PathBuf::from(home).join(".config"));
		}
		None
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::data::model::{
		DefaultViewMode, PidFilterMode, ResourceViewMode, SortColumn, Theme,
		VramPolling,
	};

	#[test]
	fn test_interface_config_default() {
		let cfg = InterfaceConfig::default();
		assert_eq!(cfg.refresh_ms, 1500);
		assert_eq!(cfg.theme, Theme::System);
	}

	#[test]
	fn test_general_config_default() {
		let cfg = GeneralConfig::default();
		assert_eq!(cfg.interface, InterfaceConfig::default());
	}

	#[test]
	fn test_column_entry_default() {
		let entry = ColumnEntry::default();
		assert_eq!(entry.column, SortColumn::Name);
		assert!(entry.visible);
	}

	#[test]
	fn test_sort_config_default() {
		let sort = SortConfig::default();
		assert_eq!(sort.column, SortColumn::Cpu);
		assert!(sort.descending);
	}

	#[test]
	fn test_behaviour_config_default() {
		let b = BehaviourConfig::default();
		assert_eq!(b.vram_polling, VramPolling::Auto);
		assert_eq!(b.pid_filter_mode, PidFilterMode::DirectChildren);
		assert!(b.clear_search_on_pin);
		assert_eq!(b.resource_view_mode, ResourceViewMode::SelfOnly);
		assert_eq!(b.default_view_mode, DefaultViewMode::List);
	}

	#[test]
	fn test_processes_config_default() {
		let cfg = ProcessesConfig::default();
		assert_eq!(cfg.behaviour, BehaviourConfig::default());
		assert_eq!(cfg.default_sort, SortConfig::default());
		assert_eq!(cfg.columns.len(), 9);
		for entry in &cfg.columns {
			assert!(entry.visible);
		}
		let columns: Vec<SortColumn> =
			cfg.columns.iter().map(|e| e.column).collect();
		assert_eq!(columns, SortColumn::all());
	}

	#[test]
	fn test_config_default() {
		let cfg = Config::default();
		assert_eq!(cfg.window_size, (1100, 700));
		assert_eq!(cfg.general, GeneralConfig::default());
		assert_eq!(cfg.processes, ProcessesConfig::default());
		assert!(cfg.disk_devices.is_empty());
		assert!(cfg.network_interfaces.is_empty());
	}

	#[test]
	fn test_is_col_visible_known_column() {
		let cfg = ProcessesConfig::default();
		assert!(cfg.is_col_visible(SortColumn::Name));
		assert!(cfg.is_col_visible(SortColumn::Cpu));
		assert!(cfg.is_col_visible(SortColumn::Vram));
	}

	#[test]
	fn test_is_col_visible_hidden() {
		let mut cfg = ProcessesConfig::default();
		if let Some(entry) = cfg
			.columns
			.iter_mut()
			.find(|e| e.column == SortColumn::Vram)
		{
			entry.visible = false;
		}
		assert!(!cfg.is_col_visible(SortColumn::Vram));
		assert!(cfg.is_col_visible(SortColumn::Cpu));
	}

	#[test]
	fn test_is_col_visible_missing_column_returns_true() {
		let mut cfg = ProcessesConfig::default();
		cfg.columns.retain(|e| e.column != SortColumn::Pid);
		assert!(cfg.is_col_visible(SortColumn::Pid));
	}

	#[test]
	fn test_default_column_layout() {
		let layout = default_column_layout();
		assert_eq!(layout.len(), 9);
		for entry in &layout {
			assert!(entry.visible);
		}
		let columns: Vec<SortColumn> =
			layout.iter().map(|e| e.column).collect();
		let mut expected = SortColumn::all();
		expected.sort_by_key(|c| c.to_col_index());
		let mut sorted: Vec<SortColumn> = columns.clone();
		sorted.sort_by_key(|c| c.to_col_index());
		assert_eq!(sorted, expected);
	}

	#[test]
	fn test_config_path_ends_with_gpuitop_config_ron() {
		let path = Config::config_path();
		assert!(
			path.ends_with("gpuitop/config.ron"),
			"Expected path to end with 'gpuitop/config.ron', got {:?}",
			path
		);
	}

	fn roundtrip<
		T: Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
	>(
		val: &T,
	) {
		let serialized = ron::ser::to_string_pretty(
			val,
			ron::ser::PrettyConfig::default(),
		)
		.expect("serialization failed");
		let deserialized: T =
			ron::from_str(&serialized).expect("deserialization failed");
		assert_eq!(*val, deserialized);
	}

	#[test]
	fn test_interface_config_roundtrip() {
		roundtrip(&InterfaceConfig::default());
		roundtrip(&InterfaceConfig {
			refresh_ms: 500,
			theme: Theme::Dark,
		});
	}

	#[test]
	fn test_general_config_roundtrip() {
		roundtrip(&GeneralConfig::default());
	}

	#[test]
	fn test_column_entry_roundtrip() {
		roundtrip(&ColumnEntry::default());
		roundtrip(&ColumnEntry {
			column: SortColumn::Vram,
			visible: false,
		});
	}

	#[test]
	fn test_sort_config_roundtrip() {
		roundtrip(&SortConfig::default());
		roundtrip(&SortConfig {
			column: SortColumn::Memory,
			descending: false,
		});
	}

	#[test]
	fn test_behaviour_config_roundtrip() {
		roundtrip(&BehaviourConfig::default());
		roundtrip(&BehaviourConfig {
			vram_polling: VramPolling::On,
			pid_filter_mode: PidFilterMode::AllDescendants,
			clear_search_on_pin: false,
			resource_view_mode: ResourceViewMode::Cumulative,
			default_view_mode: DefaultViewMode::Tree,
		});
	}

	#[test]
	fn test_processes_config_roundtrip() {
		roundtrip(&ProcessesConfig::default());
		let mut cfg = ProcessesConfig::default();
		cfg.columns.truncate(3);
		cfg.default_sort = SortConfig {
			column: SortColumn::Name,
			descending: false,
		};
		roundtrip(&cfg);
	}

	#[test]
	fn test_config_roundtrip() {
		roundtrip(&Config::default());
		let mut cfg = Config::default();
		cfg.window_size = (1920, 1080);
		cfg.disk_devices = vec!["sda".into(), "nvme0n1".into()];
		cfg.network_interfaces = vec!["eth0".into()];
		roundtrip(&cfg);
	}

	#[test]
	fn test_apply_override_refresh_ms() {
		let mut cfg = Config::default();
		cfg.apply_override(
			"(general: (interface: (refresh_ms: 500, theme: Dark)))",
		)
		.unwrap();
		assert_eq!(cfg.general.interface.refresh_ms, 500);
		assert_eq!(cfg.general.interface.theme, Theme::Dark);
	}

	#[test]
	fn test_apply_override_processes_view_mode() {
		let mut cfg = Config::default();
		cfg.processes.behaviour.default_view_mode = DefaultViewMode::List;
		cfg.apply_override(
			"(processes: (behaviour: (default_view_mode: Tree)))",
		)
		.unwrap();
		assert_eq!(
			cfg.processes.behaviour.default_view_mode,
			DefaultViewMode::Tree
		);
	}

	#[test]
	fn test_apply_override_multiple_sections() {
		let mut cfg = Config::default();
		cfg.apply_override(
			r#"(general: (interface: (refresh_ms: 500, theme: Dark)), window_size: (1920, 1080))"#,
		)
		.unwrap();
		assert_eq!(cfg.general.interface.refresh_ms, 500);
		assert_eq!(cfg.window_size, (1920, 1080));
	}

	#[test]
	fn test_apply_override_last_wins() {
		let mut cfg = Config::default();
		cfg.apply_override("(window_size: (800, 600))").unwrap();
		cfg.apply_override("(window_size: (1024, 768))").unwrap();
		assert_eq!(cfg.window_size, (1024, 768));
	}

	#[test]
	fn test_apply_override_preserves_unspecified() {
		let mut cfg = Config::default();
		cfg.processes.behaviour.vram_polling = VramPolling::Off;
		cfg.apply_override(
			"(general: (interface: (refresh_ms: 500, theme: Dark)))",
		)
		.unwrap();
		assert_eq!(cfg.processes.behaviour.vram_polling, VramPolling::Off);
		assert_eq!(cfg.window_size, (1100, 700));
	}

	#[test]
	fn test_apply_override_partial_enum_field() {
		let mut cfg = Config::default();
		cfg.general.interface.theme = Theme::Dark;
		cfg.apply_override("(general: (interface: (refresh_ms: 500)))")
			.unwrap();
		assert_eq!(cfg.general.interface.refresh_ms, 500);
		assert_eq!(cfg.general.interface.theme, Theme::Dark);
	}

	#[test]
	fn test_apply_override_partial_behaviour() {
		let mut cfg = Config::default();
		cfg.processes.behaviour.clear_search_on_pin = false;
		cfg.apply_override(
			"(processes: (behaviour: (resource_view_mode: Cumulative)))",
		)
		.unwrap();
		assert_eq!(
			cfg.processes.behaviour.resource_view_mode,
			ResourceViewMode::Cumulative
		);
		assert_eq!(cfg.processes.behaviour.clear_search_on_pin, false);
	}
}
