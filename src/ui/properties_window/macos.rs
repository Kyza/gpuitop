use gpui::prelude::*;
use gpui::*;

use crate::data::properties::ProcessProperties;
use crate::ui::properties_window::PropertiesWindow;

pub fn body(
	_properties: Option<&ProcessProperties>,
	_dead: bool,
	_tab_index: usize,
	cx: &mut Context<PropertiesWindow>,
) -> impl IntoElement {
	div()
		.flex_1()
		.flex()
		.items_center()
		.justify_center()
		.text_color(cx.theme().muted_foreground)
		.child("Process properties are not available on this platform.")
}
