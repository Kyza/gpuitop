use super::chips::{get_state_item, render_filter_chip};
use super::state::ViewState;
use super::ProcessesTab;
use crate::model::*;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	breadcrumb::{Breadcrumb, BreadcrumbItem},
	button::{Button, ButtonVariants},
	input::{Input, InputEvent, InputState},
	menu::{DropdownMenu, PopupMenuItem},
	skeleton::Skeleton,
	table::{DataTable, TableEvent, TableState},
	ActiveTheme, Disableable, Icon, IconName, Sizable,
};

impl ProcessesTab {
	fn render_toolbar(
		&mut self,
		_window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let type_filters: Vec<Filter> = vec![
			Filter::Gui,
			Filter::User,
			Filter::System,
			Filter::Services,
			Filter::Kernel,
			Filter::Parent,
			Filter::Vram,
			Filter::Electron,
		];

		let snapshot = self.snapshot_cell.borrow().clone();
		let pid_filters: Vec<(Filter, String)> = self
			.view_state
			.borrow()
			.filters
			.iter()
			.filter(|f| matches!(f, Filter::Pid(_)))
			.map(|f| {
				let label = f.label(&snapshot.processes);
				(f.clone(), label)
			})
			.collect();
		drop(snapshot);

		let mode = self.view_state.borrow().filter_mode;

		div()
			.w_full()
			.flex()
			.flex_col()
			.bg(cx.theme().background)
			.border_b_1()
			.border_color(cx.theme().border)
			.p(px(8.0))
			.gap(px(6.0))
			.child(
				div()
					.flex()
					.flex_row()
					.gap(px(8.0))
					.items_center()
					.child(
						Input::new(self.input_state.as_ref().unwrap())
							.small()
							.w_full()
							.prefix(
								Icon::new(IconName::Search)
									.size(px(12.0))
									.text_color(cx.theme().muted_foreground),
							)
							.when(
								self.view_state.borrow().search.len() > 0,
								|input| {
									let view_state = self.view_state.clone();
									let input_state = self
										.input_state
										.as_ref()
										.unwrap()
										.clone();
									input.suffix(
										Button::new("clear-search")
											.ghost()
											.icon(
												Icon::new(IconName::Close)
													.size(px(14.0))
													.text_color(
														cx.theme()
															.muted_foreground,
													),
											)
											.small()
											.on_click(cx.listener(
												move |_, _, window, cx| {
													ViewState::mutate(
														&view_state,
														|s| {
															s.search.clear();
														},
													);
													input_state.update(
														cx,
														|state, cx| {
															state.set_value(
																String::new(),
																window,
																cx,
															);
														},
													);
													cx.notify();
												},
											)),
									)
								},
							),
					)
					.child(
						Button::new("and-or")
							.ghost()
							.label(mode.to_string())
							.small()
							.on_click(cx.listener(|this, _, _, cx| {
								this.toggle_filter_mode(cx)
							})),
					)
					.child({
						let view_label = self
							.view_state
							.borrow()
							.resource_view_mode
							.to_string();
						Button::new("resource-view")
							.ghost()
							.label(view_label)
							.small()
							.on_click(cx.listener(|this, _, _, cx| {
								this.toggle_resource_view_mode(cx)
							}))
					})
					.child({
						let view_state = self.view_state.clone();
						Button::new("state-filter")
							.ghost()
							.label("State")
							.small()
							.dropdown_menu(move |menu, _window, _cx| {
								let vs = view_state.borrow();
								let active: Vec<char> = vs
									.filters
									.iter()
									.filter_map(|f| match f {
										Filter::ProcessState(c) => Some(*c),
										_ => None,
									})
									.collect();
								drop(vs);
								let vs = view_state.clone();
								menu.item(PopupMenuItem::Label(
									"Process State".into(),
								))
								.item(get_state_item(
									'R',
									&active,
									vs.clone(),
								))
								.item(get_state_item(
									'S',
									&active,
									vs.clone(),
								))
								.item(get_state_item(
									'D',
									&active,
									vs.clone(),
								))
								.item(get_state_item(
									'Z',
									&active,
									vs.clone(),
								))
								.item(get_state_item(
									'T',
									&active,
									vs.clone(),
								))
								.item(get_state_item(
									'I',
									&active,
									vs.clone(),
								))
							})
					})
					.child({
						let pick_result = self.pick_result.clone();
						let is_picking = self.is_picking.clone();
						let picking = self
							.is_picking
							.load(std::sync::atomic::Ordering::Relaxed);
						Button::new("pick-window")
							.ghost()
							.label(if picking {
								"Picking…"
							} else {
								"Pick"
							})
							.small()
							.disabled(picking)
							.on_click(cx.listener(move |_this, _, _, cx| {
								is_picking.store(
									true,
									std::sync::atomic::Ordering::Relaxed,
								);
								let r = pick_result.clone();
								let p = is_picking.clone();
								std::thread::spawn(move || {
									for _ in 0..50 {
										std::thread::sleep(
											std::time::Duration::from_millis(
												200,
											),
										);
										if let Some(id) = crate::platform::get_focused_window_app_id() {
											*r.lock().unwrap() = Some(id);
											break;
										}
									}
									p.store(
										false,
										std::sync::atomic::Ordering::Relaxed,
									);
								});
								cx.notify();
							}))
					}),
			)
			.child(
				div()
					.flex()
					.flex_row()
					.flex_wrap()
					.gap(px(4.0))
					.children(type_filters.iter().map(|f| {
						let is_active =
							self.view_state.borrow().filters.contains(f);
						render_filter_chip(
							f,
							&f.label(&[]),
							is_active,
							true,
							cx,
						)
					}))
					.children({
						let active_states: Vec<_> = self
							.view_state
							.borrow()
							.filters
							.iter()
							.filter(|f| matches!(f, Filter::ProcessState(_)))
							.cloned()
							.collect();
						let chips: Vec<AnyElement> = active_states
							.iter()
							.map(|f| {
								render_filter_chip(
									f,
									&f.label(&[]),
									true,
									true,
									cx,
								)
							})
							.collect();
						chips
					}),
			)
			.when(!pid_filters.is_empty(), |el| {
				el.child(self.render_pid_breadcrumb(_window, cx))
			})
	}

	fn render_pid_breadcrumb(
		&self,
		_window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let snapshot = self.snapshot_cell.borrow().clone();
		let pid_filters: Vec<(Filter, String)> = self
			.view_state
			.borrow()
			.filters
			.iter()
			.filter(|f| matches!(f, Filter::Pid(_)))
			.map(|f| {
				let label = f.label(&snapshot.processes);
				(f.clone(), label)
			})
			.collect();
		drop(snapshot);

		let delegate = self.get_delegate();

		div()
			.flex()
			.flex_row()
			.flex_wrap()
			.items_center()
			.gap(px(4.0))
			.child({
				div()
					.id(ElementId::Name("clear-pid-filter".into()))
					.cursor(CursorStyle::PointingHand)
					.flex()
					.items_center()
					.justify_center()
					.rounded(px(2.0))
					.hover(|s| s.bg(cx.theme().muted.opacity(0.15)))
					.child(
						Icon::new(IconName::Close)
							.size(px(12.0))
							.text_color(cx.theme().muted_foreground),
					)
					.on_click(cx.listener(move |this, _, _, cx| {
						ViewState::mutate(&this.view_state, |s| {
							s.filters
								.retain(|f| !matches!(f, Filter::Pid(_)));
						});
						cx.notify();
					}))
			})
			.child({
				let mode_label =
					match self.view_state.borrow().pid_filter_mode {
						PidFilterMode::AllDescendants => "All",
						PidFilterMode::DirectChildren => "Direct",
					};
				div()
					.id(ElementId::Name("pid-filter-mode".into()))
					.cursor(CursorStyle::PointingHand)
					.px(px(6.0))
					.py(px(2.0))
					.rounded(px(3.0))
					.border_1()
					.border_color(cx.theme().border)
					.text_size(px(10.0))
					.text_color(cx.theme().muted_foreground)
					.hover(|s| s.bg(cx.theme().muted.opacity(0.1)))
					.child(mode_label)
					.on_click(cx.listener(|this, _, _, cx| {
						this.toggle_pid_filter_mode(cx)
					}))
			})
			.children(pid_filters.iter().map(|(f, _label)| {
				if let Filter::Pid(pid) = f {
					let chain = delegate.ancestor_chain_of(*pid);
					let child_count = delegate.count_descendants_of(*pid);
					let last_idx = chain.len().saturating_sub(1);
					let breadcrumb = Breadcrumb::new();
					let bc = chain.into_iter().enumerate().fold(
						breadcrumb,
						|bc, (i, proc)| {
							let is_last = i == last_idx;
							let name = proc
								.electron_app_name
								.as_deref()
								.unwrap_or(&proc.name);
							let label = if is_last {
								if child_count > 0 {
									format!("{} (+{})", name, child_count)
								} else {
									name.to_string()
								}
							} else {
								name.to_string()
							};
							let pid = proc.pid;
							bc.child(
								BreadcrumbItem::new(label)
									.disabled(is_last)
									.on_click({
										let view_state =
											self.view_state.clone();
										move |_, _, _| {
											ViewState::mutate(
												&view_state,
												|s| {
													s.filters.clear();
													s.filters.push(
														Filter::Pid(pid),
													);
												},
											);
										}
									}),
							)
						},
					);
					div().child(bc).into_any_element()
				} else {
					div().into_any_element()
				}
			}))
	}

	fn render_status_bar(
		&self,
		_window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let filtered_count = self.get_delegate().filtered_sorted_rows().len();
		let total_count = self.snapshot_cell.borrow().processes.len();
		let active_filters = self.view_state.borrow().filters.len();
		let mode = self.view_state.borrow().filter_mode;

		div()
			.w_full()
			.h(px(24.0))
			.px(px(8.0))
			.bg(cx.theme().background)
			.border_t_1()
			.border_color(cx.theme().border)
			.flex()
			.flex_row()
			.items_center()
			.gap(px(16.0))
			.text_size(px(11.0))
			.text_color(cx.theme().muted_foreground)
			.child(format!("{filtered_count} of {total_count}"))
			.when(active_filters > 0, |el| {
				el.child(format!(
					"{active_filters} filter{} ({})",
					if active_filters > 1 { "s" } else { "" },
					mode
				))
			})
	}

	fn render_table_component(
		&self,
		_window: &mut Window,
		_cx: &mut Context<Self>,
	) -> impl IntoElement {
		let _has_data = self.has_data;
		div().flex_grow(1.0).size_full().child(if _has_data {
			DataTable::new(self.table_state.as_ref().unwrap())
				.stripe(true)
				.bordered(false)
				.into_any_element()
		} else {
			Skeleton::new().into_any_element()
		})
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
								this.needs_clear_input = true;
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
			self._subscriptions = vec![cx.subscribe_in(
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
			self.input_state = Some(input_state);
		}

		if self.needs_clear_input {
			if let Some(ref is) = self.input_state {
				is.update(cx, |state, cx| {
					state.set_value(String::new(), window, cx);
				});
			}
			self.needs_clear_input = false;
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
			.child(self.render_table_component(window, cx))
			.child(self.render_status_bar(window, cx))
			.into_any_element()
	}
}
