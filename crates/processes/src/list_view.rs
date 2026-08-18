use crate::ProcessesTab;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{skeleton::Skeleton, table::DataTable};

impl ProcessesTab {
	#[hotpath::measure]
	pub fn render_list_view(
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
