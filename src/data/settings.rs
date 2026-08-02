use crate::data::config::Config;
use crate::data::model::{
	DefaultViewMode, PidFilterMode, ResourceViewMode, SortColumn, Theme,
	VramPolling,
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

pub fn set_default_view_mode(config: &mut Config, val: &str) {
	config.processes.behaviour.default_view_mode = match val {
		"Tree" => DefaultViewMode::Tree,
		_ => DefaultViewMode::List,
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

pub fn set_window_width(config: &mut Config, val: f64) {
	config.window_size.0 = val as u32;
}

pub fn set_window_height(config: &mut Config, val: f64) {
	config.window_size.1 = val as u32;
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::data::config::Config;
	use crate::data::model::*;
	use std::sync::atomic::AtomicU64;
	use std::sync::Arc;

	#[test]
	fn test_set_refresh_ms_valid() {
		let mut cfg = Config::default();
		let atomic =
			Arc::new(AtomicU64::new(cfg.general.interface.refresh_ms));
		set_refresh_ms(&mut cfg, &atomic, "500");
		assert_eq!(cfg.general.interface.refresh_ms, 500);
		assert_eq!(atomic.load(std::sync::atomic::Ordering::SeqCst), 500);
	}

	#[test]
	fn test_set_refresh_ms_invalid() {
		let mut cfg = Config::default();
		let original = cfg.general.interface.refresh_ms;
		let atomic = Arc::new(AtomicU64::new(original));
		set_refresh_ms(&mut cfg, &atomic, "not_a_number");
		assert_eq!(cfg.general.interface.refresh_ms, original);
		assert_eq!(
			atomic.load(std::sync::atomic::Ordering::SeqCst),
			original
		);
	}

	#[test]
	fn test_set_theme_valid() {
		let cases = vec![
			("Dark", Theme::Dark),
			("Light", Theme::Light),
			("System", Theme::System),
		];
		for (input, expected) in cases {
			let mut cfg = Config::default();
			let result = set_theme(&mut cfg, input);
			assert_eq!(result, Some(expected));
			assert_eq!(cfg.general.interface.theme, expected);
		}
	}

	#[test]
	fn test_set_theme_invalid() {
		let mut cfg = Config::default();
		let original = cfg.general.interface.theme;
		let result = set_theme(&mut cfg, "InvalidTheme");
		assert_eq!(result, None);
		assert_eq!(cfg.general.interface.theme, original);
	}

	#[test]
	fn test_set_vram_polling() {
		let mut cfg = Config::default();
		set_vram_polling(&mut cfg, "Auto");
		assert_eq!(cfg.processes.behaviour.vram_polling, VramPolling::Auto);

		set_vram_polling(&mut cfg, "On");
		assert_eq!(cfg.processes.behaviour.vram_polling, VramPolling::On);

		set_vram_polling(&mut cfg, "Off");
		assert_eq!(cfg.processes.behaviour.vram_polling, VramPolling::Off);

		set_vram_polling(&mut cfg, "garbage");
		assert_eq!(cfg.processes.behaviour.vram_polling, VramPolling::Off);
	}

	#[test]
	fn test_set_pid_filter_mode() {
		let mut cfg = Config::default();
		set_pid_filter_mode(&mut cfg, "Direct only");
		assert_eq!(
			cfg.processes.behaviour.pid_filter_mode,
			PidFilterMode::DirectChildren
		);

		set_pid_filter_mode(&mut cfg, "All descendants");
		assert_eq!(
			cfg.processes.behaviour.pid_filter_mode,
			PidFilterMode::AllDescendants
		);

		set_pid_filter_mode(&mut cfg, "anything else");
		assert_eq!(
			cfg.processes.behaviour.pid_filter_mode,
			PidFilterMode::AllDescendants
		);
	}

	#[test]
	fn test_set_clear_search_on_pin() {
		let mut cfg = Config::default();
		set_clear_search_on_pin(&mut cfg, false);
		assert!(!cfg.processes.behaviour.clear_search_on_pin);
		set_clear_search_on_pin(&mut cfg, true);
		assert!(cfg.processes.behaviour.clear_search_on_pin);
	}

	#[test]
	fn test_set_resource_view_mode() {
		let mut cfg = Config::default();
		set_resource_view_mode(&mut cfg, "Self");
		assert_eq!(
			cfg.processes.behaviour.resource_view_mode,
			ResourceViewMode::SelfOnly
		);

		set_resource_view_mode(&mut cfg, "Cumulative");
		assert_eq!(
			cfg.processes.behaviour.resource_view_mode,
			ResourceViewMode::Cumulative
		);

		set_resource_view_mode(&mut cfg, "other");
		assert_eq!(
			cfg.processes.behaviour.resource_view_mode,
			ResourceViewMode::Cumulative
		);
	}

	#[test]
	fn test_set_default_view_mode() {
		let mut cfg = Config::default();
		set_default_view_mode(&mut cfg, "Tree");
		assert_eq!(
			cfg.processes.behaviour.default_view_mode,
			DefaultViewMode::Tree
		);

		set_default_view_mode(&mut cfg, "List");
		assert_eq!(
			cfg.processes.behaviour.default_view_mode,
			DefaultViewMode::List
		);

		set_default_view_mode(&mut cfg, "garbage");
		assert_eq!(
			cfg.processes.behaviour.default_view_mode,
			DefaultViewMode::List
		);
	}

	#[test]
	fn test_set_sort_column_all_display_names() {
		let cases = vec![
			("Name", SortColumn::Name),
			("PID", SortColumn::Pid),
			("User", SortColumn::User),
			("State", SortColumn::State),
			("CPU%", SortColumn::Cpu),
			("Memory%", SortColumn::Memory),
			("VRAM", SortColumn::Vram),
			("Disk R", SortColumn::DiskRead),
			("Disk W", SortColumn::DiskWrite),
		];
		for (input, expected) in cases {
			let mut cfg = Config::default();
			set_sort_column(&mut cfg, input);
			assert_eq!(
				cfg.processes.default_sort.column, expected,
				"failed for input: {input}"
			);
		}
	}

	#[test]
	fn test_set_sort_column_unknown_does_not_mutate() {
		let mut cfg = Config::default();
		let original = cfg.processes.default_sort.column;
		set_sort_column(&mut cfg, "Unknown");
		assert_eq!(cfg.processes.default_sort.column, original);
	}

	#[test]
	fn test_set_sort_descending() {
		let mut cfg = Config::default();
		set_sort_descending(&mut cfg, false);
		assert!(!cfg.processes.default_sort.descending);
		set_sort_descending(&mut cfg, true);
		assert!(cfg.processes.default_sort.descending);
	}

	#[test]
	fn test_toggle_column_visibility() {
		let mut cfg = Config::default();
		let col = SortColumn::Name;
		assert!(
			cfg.processes
				.columns
				.iter()
				.find(|e| e.column == col)
				.unwrap()
				.visible
		);
		toggle_column_visibility(&mut cfg, col);
		assert!(
			!cfg.processes
				.columns
				.iter()
				.find(|e| e.column == col)
				.unwrap()
				.visible
		);
		toggle_column_visibility(&mut cfg, col);
		assert!(
			cfg.processes
				.columns
				.iter()
				.find(|e| e.column == col)
				.unwrap()
				.visible
		);
	}

	#[test]
	fn test_toggle_column_visibility_unknown_col_does_nothing() {
		let mut cfg = Config::default();
		let before: Vec<_> = cfg
			.processes
			.columns
			.iter()
			.map(|e| (e.column, e.visible))
			.collect();
		toggle_column_visibility(&mut cfg, SortColumn::Name);
		toggle_column_visibility(&mut cfg, SortColumn::Name);
		let after: Vec<_> = cfg
			.processes
			.columns
			.iter()
			.map(|e| (e.column, e.visible))
			.collect();
		assert_eq!(before, after);
	}

	#[test]
	fn test_swap_columns_within_bounds() {
		let mut cfg = Config::default();
		let first_col = cfg.processes.columns[0].column;
		let second_col = cfg.processes.columns[1].column;
		swap_columns(&mut cfg, 0, 1);
		assert_eq!(cfg.processes.columns[0].column, second_col);
		assert_eq!(cfg.processes.columns[1].column, first_col);
	}

	#[test]
	fn test_swap_columns_out_of_bounds() {
		let mut cfg = Config::default();
		let before: Vec<_> =
			cfg.processes.columns.iter().map(|e| e.column).collect();
		let len = cfg.processes.columns.len();
		swap_columns(&mut cfg, 0, len);
		swap_columns(&mut cfg, len, 0);
		swap_columns(&mut cfg, len, len + 1);
		let after: Vec<_> =
			cfg.processes.columns.iter().map(|e| e.column).collect();
		assert_eq!(before, after);
	}
}
