use crate::data::config::Config;
use crate::data::model::{
	PidFilterMode, ResourceViewMode, SortColumn, Theme, VramPolling,
};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

pub fn set_refresh_ms(
	config: &mut Config,
	refresh_ms: &Arc<AtomicU64>,
	val: &str,
) {
	use std::sync::atomic::Ordering;
	if let Ok(ms) = val.parse::<u64>() {
		config.general.interface.refresh_ms = ms;
		refresh_ms.store(ms, Ordering::SeqCst);
	}
}

pub fn set_theme(config: &mut Config, val: &str) -> Option<Theme> {
	let new = match val {
		"Dark" => Theme::Dark,
		"Light" => Theme::Light,
		"System" => Theme::System,
		_ => return None,
	};
	config.general.interface.theme = new;
	Some(new)
}

pub fn set_vram_polling(config: &mut Config, val: &str) {
	config.processes.behaviour.vram_polling = match val {
		"Auto" => VramPolling::Auto,
		"On" => VramPolling::On,
		_ => VramPolling::Off,
	};
}

pub fn set_pid_filter_mode(config: &mut Config, val: &str) {
	config.processes.behaviour.pid_filter_mode = match val {
		"Direct only" => PidFilterMode::DirectChildren,
		_ => PidFilterMode::AllDescendants,
	};
}

pub fn set_clear_search_on_pin(config: &mut Config, val: bool) {
	config.processes.behaviour.clear_search_on_pin = val;
}

pub fn set_resource_view_mode(config: &mut Config, val: &str) {
	config.processes.behaviour.resource_view_mode = match val {
		"Self" => ResourceViewMode::SelfOnly,
		_ => ResourceViewMode::Cumulative,
	};
}

pub fn set_sort_column(config: &mut Config, val: &str) {
	config.processes.default_sort.column = match val {
		"Name" => SortColumn::Name,
		"PID" => SortColumn::Pid,
		"User" => SortColumn::User,
		"State" => SortColumn::State,
		"CPU%" => SortColumn::Cpu,
		"Memory%" => SortColumn::Memory,
		"VRAM" => SortColumn::Vram,
		"Disk R" => SortColumn::DiskRead,
		"Disk W" => SortColumn::DiskWrite,
		_ => return,
	};
}

pub fn set_sort_descending(config: &mut Config, val: bool) {
	config.processes.default_sort.descending = val;
}

pub fn toggle_column_visibility(config: &mut Config, col: SortColumn) {
	if let Some(entry) = config
		.processes
		.columns
		.iter_mut()
		.find(|e| e.column == col)
	{
		entry.visible = !entry.visible;
	}
}

pub fn swap_columns(config: &mut Config, a: usize, b: usize) {
	let n = config.processes.columns.len();
	if a < n && b < n {
		config.processes.columns.swap(a, b);
	}
}
