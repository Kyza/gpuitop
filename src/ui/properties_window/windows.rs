use gpui::*;

pub fn body(_pid: i32) -> impl IntoElement {
	div()
		.flex_1()
		.flex()
		.items_center()
		.justify_center()
		.child("WIP")
}
