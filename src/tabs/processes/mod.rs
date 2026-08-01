pub mod chips;
pub mod delegate;
pub mod fuzzy;
pub mod render;
pub mod state;
pub mod table;
#[cfg(test)]
mod tests;
pub mod theme;

use crate::config::Config;
use crate::model::*;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{input::InputState, table::TableState};
use std::cell::RefCell;
use std::rc::Rc;

pub use self::delegate::ProcessTableDelegate;

pub struct ProcessesTab {
	pub snapshot_cell: Rc<RefCell<Rc<SystemSnapshot>>>,
	pub cum_cache: Rc<
		RefCell<
			Option<
				std::collections::HashMap<i32, state::CumulativeResources>,
			>,
		>,
	>,
	pub pid_index: Rc<RefCell<Option<std::collections::HashMap<i32, usize>>>>,
	pub view_state: Rc<RefCell<state::ViewState>>,
	pub table_state: Option<Entity<TableState<ProcessTableDelegate>>>,
	pub input_state: Option<Entity<InputState>>,
	pub clear_search_on_pin: bool,
	pub needs_clear_input: bool,
	pub needs_set_input: Option<String>,
	pub needs_focus_input: bool,
	pub _events: Option<Subscription>,
	pub _subscriptions: Vec<Subscription>,
	pub has_data: bool,
	pub column_visibility: crate::config::ProcessesConfig,
	pub is_picking: std::sync::Arc<std::sync::atomic::AtomicBool>,
	pub pick_result: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl ProcessesTab {
	pub fn new(
		config: Config,
		snapshot: Rc<SystemSnapshot>,
		_cx: &mut Context<Self>,
	) -> Self {
		let sort_col = config.processes.default_sort.column.to_col_index();
		let sort_dir = if config.processes.default_sort.descending {
			gpui_component::table::ColumnSort::Descending
		} else {
			gpui_component::table::ColumnSort::Ascending
		};
		Self {
			snapshot_cell: Rc::new(RefCell::new(snapshot)),
			cum_cache: Rc::new(RefCell::new(None)),
			pid_index: Rc::new(RefCell::new(None)),
			view_state: Rc::new(RefCell::new(state::ViewState {
				generation: 0,
				filters: Vec::new(),
				filter_mode: FilterMode::And,
				pid_filter_mode: config.processes.behaviour.pid_filter_mode,
				search: String::new(),
				sort_col,
				sort_dir,
				resource_view_mode: config
					.processes
					.behaviour
					.resource_view_mode,
				cached: None,
			})),
			table_state: None,
			input_state: None,
			clear_search_on_pin: config
				.processes
				.behaviour
				.clear_search_on_pin,
			needs_clear_input: false,
			needs_set_input: None,
			needs_focus_input: false,
			_events: None,
			_subscriptions: Vec::new(),
			has_data: false,
			column_visibility: config.processes.clone(),
			is_picking: std::sync::Arc::new(
				std::sync::atomic::AtomicBool::new(false),
			),
			pick_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
		}
	}

	pub fn set_snapshot(&mut self, snapshot: Rc<SystemSnapshot>) {
		*self.snapshot_cell.borrow_mut() = snapshot;
		*self.cum_cache.borrow_mut() = None;
		*self.pid_index.borrow_mut() = None;
	}

	pub fn toggle_filter(&mut self, filter: Filter, cx: &mut Context<Self>) {
		state::ViewState::mutate(&self.view_state, |s| {
			if let Some(pos) = s.filters.iter().position(|f| *f == filter) {
				s.filters.remove(pos);
			} else {
				s.filters.push(filter);
			}
		});
		cx.notify();
	}

	pub fn toggle_filter_mode(&mut self, cx: &mut Context<Self>) {
		state::ViewState::mutate(&self.view_state, |s| {
			s.filter_mode = match s.filter_mode {
				FilterMode::And => FilterMode::Or,
				FilterMode::Or => FilterMode::And,
			};
		});
		cx.notify();
	}

	pub fn toggle_pid_filter_mode(&mut self, cx: &mut Context<Self>) {
		state::ViewState::mutate(&self.view_state, |s| {
			s.pid_filter_mode = match s.pid_filter_mode {
				PidFilterMode::AllDescendants => {
					PidFilterMode::DirectChildren
				}
				PidFilterMode::DirectChildren => {
					PidFilterMode::AllDescendants
				}
			};
		});
		cx.notify();
	}

	pub fn focus_input(&mut self, _cx: &mut Context<Self>) {
		self.needs_focus_input = true;
	}

	pub fn toggle_resource_view_mode(&mut self, cx: &mut Context<Self>) {
		state::ViewState::mutate(&self.view_state, |s| {
			s.resource_view_mode = match s.resource_view_mode {
				ResourceViewMode::SelfOnly => ResourceViewMode::Cumulative,
				ResourceViewMode::Cumulative => ResourceViewMode::SelfOnly,
			};
		});
		cx.notify();
	}

	pub fn get_delegate(&self) -> ProcessTableDelegate {
		ProcessTableDelegate {
			snapshot_cell: self.snapshot_cell.clone(),
			cum_cache: self.cum_cache.clone(),
			pid_index: self.pid_index.clone(),
			view_state: self.view_state.clone(),
			column_visibility: self.column_visibility.clone(),
		}
	}
}
