use bytesize::ByteSize;
use gpui::prelude::*;
use gpui::*;
use gpui_component::chart::{LineChart, PieChart};
use gpui_component::progress::Progress;
use gpui_component::ActiveTheme;

use gpuitop_core::model::MemoryInfo;

use super::history::Sample;
use super::widgets::{card, chart_box, stat_tiles};
use super::PerformanceTab;

#[derive(Clone)]
struct MemSlice {
	name: SharedString,
	bytes: f64,
	color: Hsla,
}

#[hotpath::measure]
pub fn memory_tab(
	mem: &MemoryInfo,
	samples: Vec<Sample>,
	cx: &mut Context<PerformanceTab>,
) -> impl IntoElement {
	let total = mem.total.max(1);
	let used_pct = mem.used as f32 / total as f32 * 100.0;
	let free = total.saturating_sub(mem.used + mem.cached);

	let slices = vec![
		MemSlice {
			name: "Used".into(),
			bytes: mem.used as f64,
			color: cx.theme().chart_1,
		},
		MemSlice {
			name: "Cached".into(),
			bytes: mem.cached as f64,
			color: cx.theme().chart_2,
		},
		MemSlice {
			name: "Free".into(),
			bytes: free as f64,
			color: cx.theme().chart_3,
		},
	];

	div()
		.flex()
		.flex_col()
		.gap(px(14.0))
		.child(
			card("Memory", cx)
				.child(
					div()
						.flex()
						.flex_col()
						.gap(px(6.0))
						.child(
							div()
								.flex()
								.flex_row()
								.justify_between()
								.child(
									div()
										.text_size(px(12.0))
										.text_color(
											cx.theme().muted_foreground,
										)
										.child("Used"),
								)
								.child(
									div()
										.text_size(px(12.0))
										.font_weight(FontWeight::SEMIBOLD)
										.text_color(cx.theme().foreground)
										.child(format!(
											"{} / {} ({:.1}%)",
											ByteSize::b(mem.used),
											ByteSize::b(mem.total),
											used_pct
										)),
								),
						)
						.child(
							Progress::new("mem-overall")
								.value(used_pct)
								.color(load_color(used_pct, cx)),
						),
				)
				.child(
					div()
						.flex()
						.flex_row()
						.flex_wrap()
						.gap(px(20.0))
						.children(stat_tiles(
							cx,
							&[
								(
									"Total",
									ByteSize::b(mem.total).to_string(),
									cx.theme().foreground,
								),
								(
									"Used",
									ByteSize::b(mem.used).to_string(),
									cx.theme().chart_1,
								),
								(
									"Available",
									ByteSize::b(mem.available).to_string(),
									cx.theme().success,
								),
								(
									"Cached",
									ByteSize::b(mem.cached).to_string(),
									cx.theme().chart_2,
								),
								(
									"Swap",
									ByteSize::b(mem.swap_used).to_string(),
									cx.theme().warning,
								),
							],
						)),
				),
		)
		.child(
			card("Composition", cx).child(
				div()
					.flex()
					.flex_row()
					.flex_wrap()
					.items_center()
					.gap(px(20.0))
					.child(
						div().w(px(200.0)).h(px(200.0)).child(
							PieChart::new(slices.clone())
								.value(|d| d.bytes as f32)
								.color(|d| d.color)
								.inner_radius(55.0)
								.outer_radius(80.0),
						),
					)
					.child(div().flex().flex_col().gap(px(10.0)).children(
						slices.iter().map(|s| slice_legend(s, cx)),
					)),
			),
		)
		.child(
			card("Usage History (MiB)", cx).child(chart_box(
				LineChart::new(samples)
					.x(|s| s.label.clone())
					.y(|s| s.mem_used / (1024.0 * 1024.0))
					.stroke(cx.theme().chart_1)
					.name("Used (MiB)")
					.tick_margin(10)
					.id("mem-history-chart"),
				"MiB",
				cx,
			)),
		)
}

fn slice_legend(
	slice: &MemSlice,
	cx: &Context<PerformanceTab>,
) -> impl IntoElement {
	div()
		.flex()
		.flex_row()
		.items_center()
		.gap(px(8.0))
		.child(
			div()
				.w(px(10.0))
				.h(px(10.0))
				.rounded(px(2.0))
				.bg(slice.color),
		)
		.child(
			div()
				.text_size(px(12.0))
				.text_color(cx.theme().muted_foreground)
				.w(px(56.0))
				.child(slice.name.clone()),
		)
		.child(
			div()
				.text_size(px(12.0))
				.font_weight(FontWeight::SEMIBOLD)
				.text_color(cx.theme().foreground)
				.child(ByteSize::b(slice.bytes as u64).to_string()),
		)
}

fn load_color(v: f32, cx: &Context<PerformanceTab>) -> Hsla {
	if v > 90.0 {
		cx.theme().danger
	} else if v > 70.0 {
		cx.theme().warning
	} else {
		cx.theme().primary
	}
}
