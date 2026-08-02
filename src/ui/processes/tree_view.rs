use crate::data::model::*;
use crate::data::state::ViewState;
use crate::ui::assets::lucide::LucideIcon;
use crate::ui::processes::context_menu;
use crate::ui::processes::ProcessesTab;
use crate::ui::theme;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	list::ListItem, scroll::ScrollableElement, tree::tree, ActiveTheme,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

impl ProcessesTab {
	pub fn render_tree_view(
		&mut self,
		_window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let delegate = self.get_delegate();
		let expanded_pids = self.expanded_pids.clone();
		let tree_data = delegate.build_tree(expanded_pids);

		self.tree_state.update(cx, |state, cx| {
			state.set_items(tree_data.items.clone(), cx);
		});

		let tree_data = Rc::new(RefCell::new(tree_data));
		let view = cx.entity();
		let view_state = self.view_state.clone();
		let clear_search_on_pin = self.clear_search_on_pin;
		let double_click = self.tree_double_click.clone();

		div()
			.flex_grow(1.0)
			.size_full()
			.flex()
			.flex_col()
			.overflow_x_scrollbar()
			.child({
				let td = tree_data.clone();
				tree(
					&self.tree_state,
					move |ix, entry, _selected, _window, cx| {
						view.update(cx, |this, cx| {
							let td = td.borrow();
							let item = entry.item();
							let id_str = item.id.to_string();
							let parts: Vec<&str> =
								id_str.split(':').collect();
							let pid_str =
								parts[0].strip_prefix("pid-").unwrap_or("0");
							let pid: i32 = pid_str.parse().unwrap_or(0);
							let is_match =
								parts.get(1).copied().unwrap_or("match")
									== "match";
							let proc = td.process_lookup.get(&pid);

							let is_folder = entry.is_folder();
							let is_expanded = entry.is_expanded();

							let mut list_item = ListItem::new(ix)
								.w_full()
								.rounded(cx.theme().radius)
								.px_3()
								.pl(px(16.) * entry.depth() + px(12.));

							if let Some(proc) = proc {
								let display_name = proc
									.electron_app_name
									.as_deref()
									.unwrap_or(&proc.name);
								let child_count = td
									.descendant_counts
									.get(&pid)
									.copied()
									.unwrap_or(0);
								let tags = theme::tag_icons(
									proc,
									this.init_system,
									cx,
								);
								list_item = list_item.child(
									div()
										.flex()
										.flex_row()
										.items_center()
										.gap(px(4.0))
										.when(!is_match, |el| el.opacity(0.4))
										.children({
											let mut children: Vec<
												AnyElement,
											> = if is_folder {
												let caret = if is_expanded {
													LucideIcon::ChevronDown
												} else {
													LucideIcon::ChevronRight
												};
												vec![caret
													.icon()
													.size(px(12.0))
													.text_color(
														cx.theme()
															.muted_foreground,
													)
													.into_any_element()]
											} else {
												vec![div()
													.w(px(12.0))
													.into_any_element()]
											};
											children.extend(
												tags.into_iter().map(
													|(icon, color)| {
														icon.icon()
															.w(px(12.0))
															.h(px(12.0))
															.text_color(color)
															.into_any_element(
															)
													},
												),
											);
											children
										})
										.child({
											if child_count > 0 {
												format!(
													"({}) {} (+{})",
													proc.pid,
													display_name,
													child_count
												)
											} else {
												format!(
													"({}) {}",
													proc.pid, display_name
												)
											}
										}),
								);
							}

							list_item.on_click(cx.listener({
								let vs = view_state.clone();
								let csp = clear_search_on_pin;
								let dc = double_click.clone();
								let id = id_str.clone();
								move |this,
								      _: &gpui::ClickEvent,
								      _window,
								      _cx| {
									let now = Instant::now();
									let mut dc_guard = dc.borrow_mut();
									if let Some((last_time, last_id)) =
										&*dc_guard
									{
										if *last_id == id
											&& now
												.duration_since(*last_time)
												< std::time::Duration::from_millis(
													400,
												) {
											ViewState::mutate(
												&vs,
												|s| {
													s.filters.clear();
													s.filters.push(
														Filter::Pid(pid),
													);
												},
											);
											if csp {
												ViewState::mutate(
													&vs,
													|s| {
														s.search.clear();
													},
												);
												this.needs_clear_input = true;
											}
											*dc_guard = None;
											return;
										}
									}
									*dc_guard = Some((now, id.clone()));
								}
							}))
						})
					},
				)
				.context_menu({
					let td = tree_data.clone();
					move |_ix, entry, mut menu, _window, _cx| {
						let td = td.borrow();
						let item = entry.item();
						let id_str = item.id.to_string();
						let parts: Vec<&str> = id_str.split(':').collect();
						let pid_str =
							parts[0].strip_prefix("pid-").unwrap_or("0");
						let pid: i32 = pid_str.parse().unwrap_or(0);
						if let Some(proc) = td.process_lookup.get(&pid) {
							for menu_item in
								context_menu::build_process_menu(proc)
							{
								menu = menu.item(menu_item);
							}
						}
						menu
					}
				})
				.flex_grow(1.0)
			})
	}
}
