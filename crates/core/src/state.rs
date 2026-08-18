use crate::model::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default, Clone)]
pub struct CumulativeResources {
	pub cpu: f32,
	pub mem_rss: u64,
	pub vram: VramUsage,
	pub disk_read: f64,
	pub disk_write: f64,
}

pub struct ViewState {
	pub generation: u64,
	pub filters: Vec<Filter>,
	pub filter_mode: FilterMode,
	pub pid_filter_mode: PidFilterMode,
	pub search: String,
	pub sort_col: usize,
	pub sort_dir: SortDirection,
	pub resource_view_mode: ResourceViewMode,
	pub cached: Option<(std::time::Instant, u64, Rc<Vec<ProcessSnapshot>>)>,
}

impl ViewState {
	pub fn mutate(
		state: &Rc<RefCell<ViewState>>,
		f: impl FnOnce(&mut ViewState),
	) {
		let mut s = state.borrow_mut();
		f(&mut s);
		s.generation = s.generation.wrapping_add(1);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::cell::RefCell;
	use std::rc::Rc;

	fn make_test_state() -> ViewState {
		ViewState {
			generation: 0,
			filters: Vec::new(),
			filter_mode: FilterMode::And,
			pid_filter_mode: PidFilterMode::AllDescendants,
			search: String::new(),
			sort_col: 0,
			sort_dir: SortDirection::Ascending,
			resource_view_mode: ResourceViewMode::SelfOnly,
			cached: None,
		}
	}

	#[test]
	fn mutate_increments_generation() {
		let state = Rc::new(RefCell::new(make_test_state()));
		assert_eq!(state.borrow().generation, 0);
		ViewState::mutate(&state, |_| {});
		assert_eq!(state.borrow().generation, 1);
		ViewState::mutate(&state, |_| {});
		assert_eq!(state.borrow().generation, 2);
		ViewState::mutate(&state, |_| {});
		assert_eq!(state.borrow().generation, 3);
	}

	#[test]
	fn mutate_wraps_generation() {
		let state = Rc::new(RefCell::new(ViewState {
			generation: u64::MAX,
			..make_test_state()
		}));
		ViewState::mutate(&state, |_| {});
		assert_eq!(state.borrow().generation, 0);
	}

	#[test]
	fn mutate_applies_mutation() {
		let state = Rc::new(RefCell::new(make_test_state()));
		assert_eq!(state.borrow().sort_col, 0);
		ViewState::mutate(&state, |s| {
			s.sort_col = 5;
		});
		assert_eq!(state.borrow().sort_col, 5);
	}

	#[test]
	fn cumulative_resources_default() {
		let cr = CumulativeResources::default();
		assert_eq!(cr.cpu, 0.0);
		assert_eq!(cr.mem_rss, 0);
		assert_eq!(cr.vram, VramUsage::default());
		assert_eq!(cr.disk_read, 0.0);
		assert_eq!(cr.disk_write, 0.0);
	}

	#[test]
	fn view_state_default() {
		let state = make_test_state();
		assert_eq!(state.generation, 0);
		assert!(state.filters.is_empty());
		assert_eq!(state.filter_mode, FilterMode::And);
		assert_eq!(state.search, "");
		assert_eq!(state.sort_col, 0);
		assert_eq!(state.sort_dir, SortDirection::Ascending);
		assert_eq!(state.resource_view_mode, ResourceViewMode::SelfOnly);
		assert!(state.cached.is_none());
	}
}
