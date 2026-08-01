use crate::model::{
	PidFilterMode, ProcessGrouping, ResourceViewMode, SortColumn, Theme,
	VramPolling,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnVisibility {
	pub name: bool,
	pub pid: bool,
	pub user: bool,
	pub state: bool,
	pub cpu: bool,
	pub memory: bool,
	pub vram: bool,
	pub disk_read: bool,
	pub disk_write: bool,
	pub command: bool,
}

impl Default for ColumnVisibility {
	fn default() -> Self {
		Self {
			name: true,
			pid: true,
			user: true,
			state: true,
			cpu: true,
			memory: true,
			vram: false,
			disk_read: true,
			disk_write: true,
			command: false,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortConfig {
	pub column: SortColumn,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	pub refresh_ms: u64,
	pub columns: ColumnVisibility,
	pub default_sort: SortConfig,
	pub default_grouping: ProcessGrouping,
	pub vram_polling: VramPolling,
	pub pid_filter_mode: PidFilterMode,
	pub resource_view_mode: ResourceViewMode,
	pub clear_search_on_pin: bool,
	pub theme: Theme,
	pub disk_devices: Vec<String>,
	pub network_interfaces: Vec<String>,
	pub window_width: u32,
	pub window_height: u32,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			refresh_ms: 1500,
			columns: ColumnVisibility::default(),
			default_sort: SortConfig::default(),
			default_grouping: ProcessGrouping::Auto,
			vram_polling: VramPolling::Auto,
			pid_filter_mode: PidFilterMode::DirectChildren,
			resource_view_mode: ResourceViewMode::SelfOnly,
			clear_search_on_pin: true,
			theme: Theme::System,
			disk_devices: Vec::new(),
			network_interfaces: Vec::new(),
			window_width: 1100,
			window_height: 700,
		}
	}
}

impl Config {
	fn config_path() -> PathBuf {
		let base = dirs::config_dir()
			.unwrap_or_else(|| PathBuf::from("."))
			.join("gpuitop");
		base.join("config.toml")
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
				Ok(data) => match toml::from_str(&data) {
					Ok(config) => return config,
					Err(e) => eprintln!(
						"Failed to parse config: {e}. Using defaults."
					),
				},
				Err(e) => {
					eprintln!("Failed to read config: {e}. Using defaults.")
				}
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
		let path = base.join("config.toml");
		let data = toml::to_string_pretty(self)
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
