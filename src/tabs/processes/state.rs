use crate::model::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default, Clone)]
pub struct CumulativeResources {
	pub cpu: f32,
	pub mem_rss: u64,
	pub vram: Option<u64>,
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
	pub sort_dir: gpui_component::table::ColumnSort,
	pub resource_view_mode: ResourceViewMode,
	pub cached: Option<(std::time::Instant, u64, Rc<Vec<ProcessInfo>>)>,
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
