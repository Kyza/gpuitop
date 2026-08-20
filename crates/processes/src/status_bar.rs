use crate::ProcessesTab;
use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme;

impl ProcessesTab {
	#[hotpath::measure]
	pub fn render_status_bar(
		&self,
		_window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let delegate = self.get_delegate();
		let filtered_count = if self.show_tree_view {
			delegate.tree_match_set().len()
		} else {
			delegate.rows().len()
		};
		let total_count = self.engine.borrow().snapshot().processes.len();
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
}
