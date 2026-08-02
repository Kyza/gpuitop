use crate::data::model::*;
use crate::data::state::ViewState;
use crate::ui::assets::lucide::LucideIcon;
use crate::ui::processes::ProcessesTab;
use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme;

impl ProcessesTab {
	pub fn render_pid_breadcrumb(
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
						LucideIcon::X
							.icon()
							.size(px(12.0))
							.text_color(cx.theme().muted_foreground),
					)
					.on_click(cx.listener(move |this, _, _, cx| {
						ViewState::mutate(&this.view_state, |s| {
							s.filters
								.retain(|f| !matches!(f, Filter::Pid(_,)));
						});
						cx.notify();
					}))
			})
			.when(!self.show_tree_view, |el| {
				el.child({
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
			})
			.children(pid_filters.iter().map(|(f, _label)| {
				if let Filter::Pid(pid) = f {
					let chain = delegate.ancestor_chain_of(*pid);
					let child_count = delegate.count_descendants_of(*pid);
					let last_idx = chain.len().saturating_sub(1);
					let breadcrumb =
						gpui_component::breadcrumb::Breadcrumb::new();
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
								gpui_component::breadcrumb::BreadcrumbItem::new(label)
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
}
