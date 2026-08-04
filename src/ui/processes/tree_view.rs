use crate::data::model::*;
use crate::data::state::ViewState;
use crate::ui::assets::lucide::LucideIcon;
use crate::ui::processes::context_menu;
use crate::ui::processes::tree::TreeData;
use crate::ui::processes::ProcessesTab;
use crate::ui::theme;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	list::ListItem, scroll::ScrollableElement, tree::tree, ActiveTheme,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

impl ProcessesTab {
	pub fn render_tree_view(
		&mut self,
		_window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let snapshot_ts = self.snapshot_cell.borrow().timestamp;
		let view_gen = self.view_state.borrow().generation;
		let stamp = (snapshot_ts, view_gen);

		let needs_rebuild = self
			.cached_tree_data
			.as_ref()
			.map_or(true, |(s, _)| *s != stamp);
		if needs_rebuild {
			let delegate = self.get_delegate();
			let mut tree_data = delegate.build_tree();
			if let Some((_, ref old_tree_data)) = self.cached_tree_data {
				tree_data.preserve_expand_from(old_tree_data);
			}
			self.tree_state.update(cx, |state, cx| {
				state.set_items(tree_data.items.clone(), cx);
			});
			self.cached_tree_data = Some((stamp, Rc::new(tree_data)));
		}

		let tree_data = self.cached_tree_data.as_ref().unwrap().1.clone();
		let view = cx.entity();
		let view_state = self.view_state.clone();
		let clear_search_on_pin = self.clear_search_on_pin;
		let double_click = self.tree_double_click.clone();
		let init_system = self.init_system;
		let needs_clear_input = self.needs_clear_input.clone();

		div()
			.flex_grow(1.0)
			.size_full()
			.flex()
			.flex_col()
			.overflow_x_scrollbar()
			.child({
				let td = tree_data.clone();
				let v = view.clone();
				let vs = view_state.clone();
				let csp = clear_search_on_pin;
				let dc = double_click.clone();
				let init = init_system;
				let nci = needs_clear_input.clone();
				tree(
					&self.tree_state,
					move |ix, entry, _selected, _window, cx| {
						Self::render_tree_node(
							ix, entry, &td, &vs, csp, &dc, init, &nci, &v, cx,
						)
					},
				)
				.context_menu({
					let td = tree_data.clone();
					move |_ix, entry, mut menu, _window, _cx| {
						let item = entry.item();
						let id_str = item.id.to_string();
						let parts: Vec<&str> = id_str.split(':').collect();
						let pid_str =
							parts[0].strip_prefix("pid-").unwrap_or("0");
						let pid: i32 = pid_str.parse().unwrap_or(0);
						if let Some(proc) = td.process_lookup.get(&pid) {
							for mi in context_menu::build_process_menu(proc) {
								menu = menu.item(mi);
							}
						}
						menu
					}
				})
				.flex_grow(1.0)
			})
	}

	fn render_tree_node(
		ix: usize,
		entry: &gpui_component::tree::TreeEntry,
		tree_data: &TreeData,
		view_state: &Rc<RefCell<ViewState>>,
		clear_search_on_pin: bool,
		double_click: &Rc<RefCell<Option<(Instant, String)>>>,
		init_system: crate::data::service_manager::InitSystem,
		needs_clear_input: &Rc<Cell<bool>>,
		view: &Entity<ProcessesTab>,
		_app: &mut App,
	) -> ListItem {
		let item = entry.item();
		let id_str = item.id.to_string();
		let parts: Vec<&str> = id_str.split(':').collect();
		let pid_str = parts[0].strip_prefix("pid-").unwrap_or("0");
		let pid: i32 = pid_str.parse().unwrap_or(0);
		let is_match = parts.get(1).copied().unwrap_or("match") == "match";

		let proc = tree_data.process_lookup.get(&pid);
		let child_count =
			tree_data.descendant_counts.get(&pid).copied().unwrap_or(0);

		let is_folder = entry.is_folder();
		let is_expanded = entry.is_expanded();

		let mut list_item = ListItem::new(ix)
			.w_full()
			.rounded(_app.theme().radius)
			.px_3()
			.pl(px(16.) * entry.depth() + px(12.));

		if let Some(proc) = proc {
			let display_name =
				proc.electron_app_name.as_deref().unwrap_or(&proc.name);
			let tags = theme::tag_icons(proc, init_system, _app);
			list_item = list_item.child(
				div()
					.flex()
					.flex_row()
					.items_center()
					.gap(px(4.0))
					.when(!is_match, |el| el.opacity(0.4))
					.children({
						let mut children: Vec<AnyElement> = if is_folder {
							let caret = if is_expanded {
								LucideIcon::ChevronDown
							} else {
								LucideIcon::ChevronRight
							};
							vec![caret
								.icon()
								.size(px(12.0))
								.text_color(_app.theme().muted_foreground)
								.into_any_element()]
						} else {
							vec![div().w(px(12.0)).into_any_element()]
						};
						children.extend(tags.into_iter().map(
							|(icon, color)| {
								icon.icon()
									.w(px(12.0))
									.h(px(12.0))
									.text_color(color)
									.into_any_element()
							},
						));
						children
					})
					.child({
						if child_count > 0 {
							format!(
								"({}) {} (+{})",
								proc.pid, display_name, child_count
							)
						} else {
							format!("({}) {}", proc.pid, display_name)
						}
					}),
			);
		}

		list_item.on_click({
			let vs = view_state.clone();
			let dc = double_click.clone();
			let nci = needs_clear_input.clone();
			let v = view.clone();
			let id = id_str;
			move |_event: &gpui::ClickEvent, _window, cx| {
				let now = Instant::now();
				let mut dc_guard = dc.borrow_mut();
				if let Some((last_time, last_id)) = &*dc_guard {
					if *last_id == id
						&& now.duration_since(*last_time)
							< std::time::Duration::from_millis(400)
					{
						ViewState::mutate(&vs, |s| {
							s.filters.clear();
							s.filters.push(Filter::Pid(pid));
						});
						if clear_search_on_pin {
							ViewState::mutate(&vs, |s| {
								s.search.clear();
							});
							nci.set(true);
						}
						v.update(cx, |_, cx| {
							cx.notify();
						});
						*dc_guard = None;
						return;
					}
				}
				*dc_guard = Some((now, id.clone()));
			}
		})
	}
}
