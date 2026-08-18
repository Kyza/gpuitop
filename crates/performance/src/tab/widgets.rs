use bytesize::ByteSize;
use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme;

use super::PerformanceTab;

pub fn card(title: &str, cx: &Context<PerformanceTab>) -> Div {
	div()
		.w_full()
		.flex()
		.flex_col()
		.gap(px(12.0))
		.rounded(cx.theme().radius_lg)
		.border_1()
		.border_color(cx.theme().border)
		.p(px(14.0))
		.child(
			div()
				.text_size(px(13.0))
				.font_weight(FontWeight::SEMIBOLD)
				.text_color(cx.theme().foreground)
				.child(title.to_string()),
		)
}

pub fn stat(
	label: &str,
	value: String,
	color: Hsla,
	cx: &Context<PerformanceTab>,
) -> impl IntoElement {
	div()
		.flex()
		.flex_col()
		.gap(px(2.0))
		.child(
			div()
				.text_size(px(11.0))
				.text_color(cx.theme().muted_foreground)
				.child(label.to_string()),
		)
		.child(
			div()
				.text_size(px(16.0))
				.font_weight(FontWeight::SEMIBOLD)
				.text_color(color)
				.child(value),
		)
}

pub fn stat_tiles(
	cx: &Context<PerformanceTab>,
	stats: &[(&str, String, Hsla)],
) -> Vec<AnyElement> {
	stats
		.iter()
		.map(|(l, v, c)| stat(l, v.clone(), *c, cx).into_any_element())
		.collect()
}

pub fn chart_box(
	chart: impl IntoElement,
	unit: &str,
	cx: &Context<PerformanceTab>,
) -> impl IntoElement {
	div()
		.w_full()
		.h(px(160.0))
		.relative()
		.child(
			div()
				.absolute()
				.top_0()
				.left_0()
				.text_size(px(10.0))
				.text_color(cx.theme().muted_foreground)
				.child(unit.to_string()),
		)
		.child(chart)
}

pub fn format_rate(bytes_per_sec: f64) -> String {
	if bytes_per_sec < 1024.0 {
		"0 B/s".into()
	} else {
		format!("{}/s", ByteSize::b(bytes_per_sec as u64))
	}
}
