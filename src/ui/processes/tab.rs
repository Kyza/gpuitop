use crate::data::config::Config;
use crate::data::model::Filter;
use crate::data::model::*;
use crate::data::platform::system::InitSystem;
use crate::data::processes::delegate::ProcessTableDelegate;
use crate::data::state::ViewState;
use crate::ui::processes::tree::TreeData;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	input::{InputEvent, InputState},
	table::{TableEvent, TableState},
	tree::TreeState,
	ActiveTheme,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

pub struct ProcessesTab {
	pub snapshot_cell: Rc<RefCell<Rc<SystemSnapshot>>>,
	pub cum_cache: Rc<
		RefCell<
			Option<
				std::collections::HashMap<
					i32,
					crate::data::state::CumulativeResources,
				>,
			>,
		>,
	>,
	pub pid_index: Rc<RefCell<Option<std::collections::HashMap<i32, usize>>>>,
	pub descendant_counts:
		Rc<RefCell<Option<std::collections::HashMap<i32, usize>>>>,
	pub view_state: Rc<RefCell<ViewState>>,
	pub table_state: Option<Entity<TableState<ProcessTableDelegate>>>,
	pub input_state: Option<Entity<InputState>>,
	pub clear_search_on_pin: bool,
	pub needs_clear_input: Rc<Cell<bool>>,
	pub needs_set_input: Option<String>,
	pub needs_focus_input: bool,
	pub _events: Option<Subscription>,
	pub _subscriptions: Vec<Subscription>,
	pub has_data: bool,
	pub column_visibility: crate::data::config::ProcessesConfig,
	pub is_picking: std::sync::Arc<std::sync::atomic::AtomicBool>,
	pub pick_result: std::sync::Arc<std::sync::Mutex<Option<String>>>,
	pub init_system: InitSystem,
	pub show_filters: bool,
	pub show_tree_view: bool,
	pub tree_state: Entity<TreeState>,
	pub tree_double_click: Rc<RefCell<Option<(Instant, String)>>>,
	pub cached_tree_data: Option<((std::time::Instant, u64), Rc<TreeData>)>,
}

impl ProcessesTab {
	pub fn new(
		config: Config,
		snapshot: Rc<SystemSnapshot>,
		init_system: InitSystem,
		override_view: Option<bool>,
		search: Option<String>,
		cx: &mut Context<Self>,
	) -> Self {
		let sort_col = config.processes.default_sort.column.to_col_index();
		let sort_dir = if config.processes.default_sort.descending {
			crate::data::model::SortDirection::Descending
		} else {
			crate::data::model::SortDirection::Ascending
		};
		let show_tree = override_view.unwrap_or(
			config.processes.behaviour.default_view_mode
				== DefaultViewMode::Tree,
		);
		Self {
			snapshot_cell: Rc::new(RefCell::new(snapshot)),
			cum_cache: Rc::new(RefCell::new(None)),
			pid_index: Rc::new(RefCell::new(None)),
			descendant_counts: Rc::new(RefCell::new(None)),
			view_state: Rc::new(RefCell::new(ViewState {
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
			needs_clear_input: Rc::new(Cell::new(false)),
			needs_set_input: search,
			needs_focus_input: false,
			_events: None,
			_subscriptions: Vec::new(),
			has_data: false,
			column_visibility: config.processes.clone(),
			is_picking: std::sync::Arc::new(
				std::sync::atomic::AtomicBool::new(false),
			),
			pick_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
			init_system,
			show_filters: true,
			show_tree_view: show_tree,
			tree_state: cx.new(|cx| TreeState::new(cx)),
			tree_double_click: Rc::new(RefCell::new(None)),
			cached_tree_data: None,
		}
	}

	pub fn set_snapshot(&mut self, snapshot: Rc<SystemSnapshot>) {
		*self.snapshot_cell.borrow_mut() = snapshot;
		*self.cum_cache.borrow_mut() = None;
		*self.pid_index.borrow_mut() = None;
		*self.descendant_counts.borrow_mut() = None;
	}

	pub fn toggle_filter_mode(&mut self, cx: &mut Context<Self>) {
		ViewState::mutate(&self.view_state, |s| {
			s.filter_mode = match s.filter_mode {
				FilterMode::And => FilterMode::Or,
				FilterMode::Or => FilterMode::And,
			};
		});
		cx.notify();
	}

	pub fn toggle_pid_filter_mode(&mut self, cx: &mut Context<Self>) {
		ViewState::mutate(&self.view_state, |s| {
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
		ViewState::mutate(&self.view_state, |s| {
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
			descendant_counts: self.descendant_counts.clone(),
			view_state: self.view_state.clone(),
			column_visibility: self.column_visibility.clone(),
			init_system: self.init_system,
		}
	}
}

impl Render for ProcessesTab {
	fn render(
		&mut self,
		window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let snapshot = self.snapshot_cell.borrow().clone();
		self.has_data = !snapshot.processes.is_empty();
		drop(snapshot);

		if let Some(app_id) = self.pick_result.lock().unwrap().take() {
			self.is_picking
				.store(false, std::sync::atomic::Ordering::Relaxed);
			let keyword =
				app_id.rsplit('.').next().unwrap_or(&app_id).to_string();
			self.needs_set_input = Some(keyword);
			cx.notify();
		}

		if self.table_state.is_none() {
			let delegate = self.get_delegate();
			let state = cx.new(|cx| TableState::new(delegate, window, cx));
			let events = cx.subscribe(
				&state,
				|this: &mut ProcessesTab, _state, event: &TableEvent, cx| {
					if let TableEvent::DoubleClickedRow(row_ix) = event {
						let pid = this
							.get_delegate()
							.filtered_sorted_rows()
							.get(*row_ix)
							.map(|p| p.pid);
						if let Some(pid) = pid {
							ViewState::mutate(&this.view_state, |s| {
								s.filters.clear();
								s.filters.push(Filter::Pid(pid));
							});
							if this.clear_search_on_pin {
								ViewState::mutate(&this.view_state, |s| {
									s.search.clear();
								});
								this.needs_clear_input.set(true);
							}
							cx.notify();
						}
					}
				},
			);
			self._events = Some(events);
			self.table_state = Some(state);
			cx.notify();
		}

		let _table_state = self.table_state.as_ref().unwrap();

		if self.input_state.is_none() {
			let input_state = cx.new(|cx| {
				InputState::new(window, cx)
					.placeholder("Search by name, PID, user...")
			});
			let view_state = self.view_state.clone();
			let is_clone = input_state.clone();
			let subs: Vec<Subscription> = vec![cx.subscribe_in(
				&input_state,
				window,
				move |_, _, ev: &InputEvent, _, cx| match ev {
					InputEvent::Change => {
						let value = is_clone.read(cx).value();
						ViewState::mutate(&view_state, |s| {
							s.search = value.to_string();
						});
					}
					_ => {}
				},
			)];

			self._subscriptions = subs;
			self.input_state = Some(input_state);
		}

		if self.needs_clear_input.get() {
			if let Some(ref is) = self.input_state {
				is.update(cx, |state, cx| {
					state.set_value(String::new(), window, cx);
				});
			}
			self.needs_clear_input.set(false);
		}

		if let Some(value) = self.needs_set_input.take() {
			if let Some(ref is) = self.input_state {
				let vs = self.view_state.clone();
				is.update(cx, |state, cx| {
					state.set_value(value.clone(), window, cx);
				});
				ViewState::mutate(&vs, |s| {
					s.search = value;
				});
			}
		}

		if self.needs_focus_input {
			self.needs_focus_input = false;
			if let Some(ref is) = self.input_state {
				let is_clone = is.clone();
				cx.on_next_frame(window, move |_, w, cx| {
					is_clone.update(cx, |state, cx| {
						state.focus(w, cx);
					});
				});
			}
		}

		div()
			.size_full()
			.flex()
			.flex_col()
			.bg(cx.theme().background)
			.child(self.render_toolbar(window, cx))
			.child(if self.show_tree_view {
				self.render_tree_view(window, cx).into_any_element()
			} else {
				self.render_list_view(window, cx).into_any_element()
			})
			.child(self.render_status_bar(window, cx))
			.into_any_element()
	}
}
