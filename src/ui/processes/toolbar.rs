use crate::data::model::*;
use crate::data::state::ViewState;
use crate::ui::assets::lucide::LucideIcon;
use crate::ui::processes::ProcessesTab;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	button::{ButtonVariants, ToggleVariants},
	menu::DropdownMenu,
	ActiveTheme, Disableable, Sizable,
};

impl ProcessesTab {
	pub fn render_toolbar(
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
		let mut username_set = std::collections::BTreeSet::new();
		for p in &snapshot.processes {
			if !p.user.is_empty() {
				username_set.insert(p.user.clone());
			}
		}
		let unique_usernames: Vec<String> =
			username_set.into_iter().collect();
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
					gpui_component::button::Toggle::new("toggle-filters")
						.outline()
						.checked(self.show_filters)
						.icon(
							LucideIcon::Menu.icon()
								.size(px(16.0))
								.text_color(
									cx.theme()
										.muted_foreground,
								),
						)
						.tooltip(if self.show_filters {
							"Hide filter bar."
						} else {
							"Show filter bar."
						})
						.on_click(cx.listener(
							move |this, checked: &bool, _, cx| {
								this.show_filters = *checked;
								cx.notify();
							},
						)),
				)
				.child({
					let show_tree = self.show_tree_view;
					gpui_component::button::Button::new("toggle-view-mode")
						.outline()
						.icon(
							if show_tree {
								LucideIcon::ListTree.icon()
							} else {
								LucideIcon::Rows3.icon()
							}
							.size(px(16.0))
							.text_color(
								cx.theme().muted_foreground,
							),
						)
						.tooltip(if show_tree {
							"Switch to list view."
						} else {
							"Switch to tree view."
						})
						.on_click(cx.listener(
							|this, _, _, cx| {
								this.show_tree_view =
									!this.show_tree_view;
								cx.notify();
							},
						))
				})
					.child({
						let pick_result = self.pick_result.clone();
						let is_picking = self.is_picking.clone();
						let picking = self
							.is_picking
							.load(std::sync::atomic::Ordering::Relaxed);
						let has_text =
							self.view_state.borrow().search.len() > 0;
						let input_state =
							self.input_state.as_ref().unwrap().clone();
						let view_state = self.view_state.clone();
						gpui_component::input::Input::new(
							self.input_state.as_ref().unwrap(),
						)
						.w_full()
						.prefix(
							LucideIcon::Search
								.icon()
								.size(px(12.0))
								.text_color(
									cx.theme().muted_foreground,
								),
						)
						.suffix(
							div()
								.flex()
								.flex_row()
								.child({
									let r = pick_result.clone();
									let p = is_picking.clone();
									gpui_component::button::Button::new(
										"pick-window",
									)
									.ghost()
									.when(picking, |this| {
										this.icon(gpui_component::spinner::Spinner::new().xsmall())
									})
									.when(!picking, |this| {
										this.icon(
												LucideIcon::Crosshair.icon()
													.size(px(12.0))
													.text_color(
														cx.theme()
															.muted_foreground,
													),
											)
									})
									.disabled(picking)
									.small()
									.tooltip(
										"Click a window to find its process.",
									)
									.on_click(cx.listener(
										move |_, _, _, cx| {
											p.store(
												true,
												std::sync::atomic::Ordering::Relaxed,
											);
											let r2 = r.clone();
											let p2 = p.clone();
											std::thread::spawn(move || {
												for _ in 0..50 {
													std::thread::sleep(
														std::time::Duration::from_millis(200),
													);
													if let Some(id) = crate::data::platform::get_focused_window_app_id() {
														*r2.lock().unwrap() = Some(id);
														break;
													}
												}
												p2.store(
													false,
													std::sync::atomic::Ordering::Relaxed,
												);
											});
											cx.notify();
										},
									))
									.into_any_element()
								})
								.when(has_text, |el| {
									el.child(
										gpui_component::button::Button::new("clear-search")
											.ghost()
											.icon(
												LucideIcon::X.icon()
													.size(px(14.0))
													.text_color(
														cx.theme()
															.muted_foreground,
													),
											)
											.small()
											.on_click(cx.listener(move |_, _, window, cx| {
												ViewState::mutate(&view_state, |s| {
													s.search.clear();
												});
												input_state.update(cx, |state, cx| {
													state.set_value(String::new(), window, cx);
												});
												cx.notify();
											})),
									)
								}),
						)
					}),
			)
			.when(self.show_filters, |el| {
				el.child(
					div()
						.flex()
						.flex_row()
						.flex_wrap()
						.gap(px(4.0))
						.items_center()
						.child({
							let active: Vec<bool> = type_filters
								.iter()
								.map(|f| {
									self.view_state
										.borrow()
										.filters
										.contains(f)
								})
								.collect();
							let mut group =
								gpui_component::button::ToggleGroup::new(
									"type-filters",
								)
								.segmented()
								.small()
								.outline();
							for (i, f) in type_filters.iter().enumerate() {
								let icon_name =
									crate::ui::theme::filter_icon(f);
								let tooltip = match f {
									Filter::Gui => {
										"Processes with a graphical window."
									}
									Filter::User => "Processes owned by you.",
									Filter::System => {
										"System processes not owned by any \
										 user."
									}
									Filter::Services => {
										"Processes managed by the init \
										 system."
									}
									Filter::Kernel => "Kernel threads.",
									Filter::Parent => {
										"Processes that have child processes."
									}
									Filter::Vram => {
										"Processes using GPU video memory."
									}
									Filter::Electron => {
										"Electron-based desktop applications."
									}
									_ => "",
								};
								let label = match f {
									Filter::Gui => "GUI",
									Filter::User => "User",
									Filter::System => "System",
									Filter::Services => "Services",
									Filter::Kernel => "Kernel",
									Filter::Parent => "Parent",
									Filter::Vram => "VRAM",
									Filter::Electron => "Electron",
									_ => "",
								};
								let id = match f {
									Filter::Gui => "gui",
									Filter::User => "user",
									Filter::System => "system",
									Filter::Services => "services",
									Filter::Kernel => "kernel",
									Filter::Parent => "parent",
									Filter::Vram => "vram",
									Filter::Electron => "electron",
									_ => "filter",
								};
								let color =
									crate::ui::theme::filter_color(f, cx);
								let mut t =
									gpui_component::button::Toggle::new(id)
										.checked(active[i])
										.tooltip(tooltip)
										.gap_1();
								if let Some(ic) = icon_name {
									t = t.icon(
										ic.icon()
											.size(px(12.0))
											.text_color(color),
									);
								}
								t = t.label(label);
								group = group.child(t);
							}
							group.on_click(cx.listener(
								move |this, checkeds: &Vec<bool>, _, cx| {
									let prev = this
										.view_state
										.borrow()
										.filters
										.clone();
									ViewState::mutate(
										&this.view_state,
										|s| {
											for (i, f) in type_filters
												.iter()
												.enumerate()
											{
												let now = checkeds
													.get(i)
													.copied()
													.unwrap_or(false);
												let was = prev
													.iter()
													.any(|pf| pf == f);
												if now != was {
													if now {
														s.filters
															.push(f.clone());
													} else {
														s.filters.retain(
															|pf| pf != f,
														);
													}
												}
											}
										},
									);
									cx.notify();
								},
							))
						}),
				)
				.when(unique_usernames.len() > 1, |el| {
					let usernames = unique_usernames.clone();
					let active: Vec<bool> = usernames
						.iter()
						.map(|u| {
							self.view_state.borrow().filters.iter().any(
								|f| matches!(f, Filter::Username(s) if s == u),
							)
						})
						.collect();
					let mut group = gpui_component::button::ToggleGroup::new(
						"username-filters",
					)
					.segmented()
					.small()
					.outline();
					for (i, u) in usernames.iter().enumerate() {
						let uname = u.clone();
						let t = gpui_component::button::Toggle::new(
							format!("username-{uname}"),
						)
						.checked(active[i])
						.tooltip(format!("Processes owned by {uname}."))
						.gap_1()
						.label(uname.clone());
						group = group.child(t);
					}
					let group = group.on_click(cx.listener(
						move |this, checkeds: &Vec<bool>, _, cx| {
							ViewState::mutate(&this.view_state, |s| {
								s.filters.retain(|f| {
									!matches!(f, Filter::Username(_))
								});
								for (i, checked) in
									checkeds.iter().enumerate()
								{
									if *checked {
										s.filters.push(Filter::Username(
											usernames[i].clone(),
										));
									}
								}
							});
							cx.notify();
						},
					));
					el.child(group)
				})
				.child({
					let active_states: Vec<char> = self
						.view_state
						.borrow()
						.filters
						.iter()
						.filter_map(|f| match f {
							Filter::ProcessState(c) => Some(*c),
							_ => None,
						})
						.collect();
					div()
						.flex()
						.flex_row()
						.gap(px(4.0))
						.child(
							gpui_component::button::ButtonGroup::new(
								"controls",
							)
							.outline()
							.child(
								gpui_component::button::Button::new("and-or")
									.label(mode.to_string())
									.small()
									.tooltip(
										"Toggle between AND and OR filter \
										 logic.",
									)
									.on_click(cx.listener(
										|this, _, _, cx| {
											this.toggle_filter_mode(cx)
										},
									)),
							)
						.child({
							let view_label = self
								.view_state
								.borrow()
								.resource_view_mode
								.to_string();
							gpui_component::button::Button::new(
								"resource-view",
							)
							.label(view_label)
							.small()
							.tooltip(
								"Toggle between per-process and \
								 cumulative resource usage.",
							)
							.on_click(cx.listener(
								|this, _, _, cx| {
									this.toggle_resource_view_mode(cx)
								},
							))
						}),
					)
						.child(
							div()
								.flex()
								.flex_row()
								.gap_1()
								.child({
									let view_state = self.view_state.clone();
									gpui_component::button::Button::new(
										"state-filter",
									)
									.label("State")
									.dropdown_caret(true)
									.small()
									.tooltip("Filter processes by state.")
									.dropdown_menu(
										move |mut menu, _window, _cx| {
											let vs = view_state.borrow();
											let active: Vec<char> = vs
												.filters
												.iter()
												.filter_map(|f| match f {
													Filter::ProcessState(
														c,
													) => Some(*c),
													_ => None,
												})
												.collect();
											drop(vs);
											let vs = view_state.clone();
											let all_states = [
												'R', 'S', 'D', 'Z', 'T', 't',
												'I',
											];
											if !active.is_empty() {
												menu = menu.item(gpui_component::menu::PopupMenuItem::Label(
													"Process State".into(),
												));
											}
											for c in all_states.iter().filter(
												|c| !active.contains(c),
											) {
												menu = menu.item(crate::ui::processes::chips::get_state_item(
													*c,
													&active,
													vs.clone(),
												));
											}
											menu
										},
									)
								})
								.when(!active_states.is_empty(), |el| {
									el.child(
										gpui_component::button::ButtonGroup::new("active-state-buttons")
											.outline()
											.children(active_states.iter().map(|c| {
										let state = *c;
										gpui_component::button::Button::new(format!("state-btn-{state}"))
											.label(state.to_string())
											.small()
											.tooltip("Click to remove this state filter.")
											.on_click(cx.listener(move |this, _, _, cx| {
												ViewState::mutate(&this.view_state, |s| {
													s.filters.retain(|f| {
														!matches!(f, Filter::ProcessState(c2) if *c2 == state)
													});
												});
												cx.notify();
											}))
									}))
								)
								}),
						)
				})
				.when(!pid_filters.is_empty(), |el| {
					el.child(self.render_pid_breadcrumb(_window, cx))
				})
			})
	}
}
