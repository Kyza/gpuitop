use crate::config::{ProcessesConfig, SortConfig};
use crate::model::DefaultViewMode;

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessSeeds {
	pub default_sort: SortConfig,
	pub default_view: DefaultViewMode,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SeedChanges {
	pub sort: bool,
	pub default_view: bool,
}

impl ProcessSeeds {
	pub fn capture(cfg: &ProcessesConfig) -> Self {
		Self {
			default_sort: cfg.default_sort.clone(),
			default_view: cfg.behaviour.default_view_mode.clone(),
		}
	}

	/// Records which seeds changed relative to the last capture, updating
	/// `self` to the current values.
	pub fn update(&mut self, cfg: &ProcessesConfig) -> SeedChanges {
		let mut changes = SeedChanges::default();
		if cfg.default_sort != self.default_sort {
			changes.sort = true;
			self.default_sort = cfg.default_sort.clone();
		}
		if cfg.behaviour.default_view_mode != self.default_view {
			changes.default_view = true;
			self.default_view = cfg.behaviour.default_view_mode.clone();
		}
		changes
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::config::Config;
	use crate::model::{SortColumn, SortDirection};

	#[test]
	fn capture_reads_both_seeds() {
		let cfg = Config::default().processes;
		let seeds = ProcessSeeds::capture(&cfg);
		assert_eq!(seeds.default_sort, cfg.default_sort);
		assert_eq!(seeds.default_view, cfg.behaviour.default_view_mode);
	}

	#[test]
	fn update_reports_no_change() {
		let cfg = Config::default().processes;
		let mut seeds = ProcessSeeds::capture(&cfg);
		let changes = seeds.update(&cfg);
		assert_eq!(changes, SeedChanges::default());
	}

	#[test]
	fn update_detects_sort_change() {
		let mut cfg = Config::default().processes;
		let mut seeds = ProcessSeeds::capture(&cfg);
		cfg.default_sort = SortConfig {
			column: SortColumn::Memory,
			descending: false,
		};
		let changes = seeds.update(&cfg);
		assert!(changes.sort);
		assert!(!changes.default_view);
		assert_eq!(seeds.default_sort, cfg.default_sort);
	}

	#[test]
	fn update_detects_view_change() {
		let mut cfg = Config::default().processes;
		let mut seeds = ProcessSeeds::capture(&cfg);
		cfg.behaviour.default_view_mode = DefaultViewMode::Tree;
		let changes = seeds.update(&cfg);
		assert!(!changes.sort);
		assert!(changes.default_view);
		assert_eq!(seeds.default_view, DefaultViewMode::Tree);
	}

	#[test]
	fn update_ignores_unrelated_fields() {
		let mut cfg = Config::default().processes;
		let mut seeds = ProcessSeeds::capture(&cfg);
		cfg.behaviour.clear_search_on_pin = false;
		cfg.columns.clear();
		let changes = seeds.update(&cfg);
		assert_eq!(changes, SeedChanges::default());
	}

	#[test]
	fn sort_direction_is_part_of_the_seed() {
		let mut cfg = Config::default().processes;
		let mut seeds = ProcessSeeds::capture(&cfg);
		cfg.default_sort.descending = false;
		let changes = seeds.update(&cfg);
		assert!(changes.sort);
		let _ = SortDirection::Ascending;
	}
}
