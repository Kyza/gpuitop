use gpui::prelude::*;
use gpui::*;
use gpui_component::chart::LineChart;
use gpui_component::progress::{Progress, ProgressCircle};
use gpui_component::{ActiveTheme, Sizable};

use gpuitop_core::model::CpuInfo;

use super::history::Sample;
use super::widgets::{card, chart_box, stat_tiles};
use super::PerformanceTab;

#[hotpath::measure]
pub fn cpu_tab(
	cpu: &CpuInfo,
	samples: Vec<Sample>,
	cx: &mut Context<PerformanceTab>,
) -> impl IntoElement {
	let cores = &cpu.cores;
	let overall = cpu.overall_percent;

	let min = cores
		.iter()
		.map(|c| c.usage_percent)
		.fold(f32::MAX, f32::min);
	let max = cores
		.iter()
		.map(|c| c.usage_percent)
		.fold(f32::MIN, f32::max);
	let avg = if cores.is_empty() {
		0.0
	} else {
		cores.iter().map(|c| c.usage_percent).sum::<f32>()
			/ cores.len() as f32
	};

	let overall_color = load_color(overall, cx);
	let history_color = cx.theme().chart_1;
	let temp_str = if cpu.temperature > 0.0 {
		format!("{:.1} °C", cpu.temperature)
	} else {
		"—".to_string()
	};

	div()
		.flex()
		.flex_col()
		.gap(px(14.0))
		.child({
			let title = if cpu.model_name.is_empty() {
				"CPU Usage".to_string()
			} else {
				cpu.model_name.clone()
			};
			card(&title, cx).child(
				div()
					.flex()
					.flex_row()
					.items_center()
					.flex_wrap()
					.gap(px(20.0))
					.child(
						ProgressCircle::new("cpu-overall")
							.with_size(px(110.0))
							.value(overall)
							.color(overall_color)
							.child(
								div()
									.flex()
									.flex_col()
									.items_center()
									.child(
										div()
											.text_size(px(22.0))
											.font_weight(FontWeight::BOLD)
											.text_color(cx.theme().foreground)
											.child(format!(
												"{:.0}%",
												overall
											)),
									)
									.child(
										div()
											.text_size(px(10.0))
											.text_color(
												cx.theme().muted_foreground,
											)
											.child("Total"),
									),
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
										"Cores",
										cores.len().to_string(),
										cx.theme().foreground,
									),
									(
										"Min",
										format!("{:.0}%", min),
										cx.theme().success,
									),
									(
										"Max",
										format!("{:.0}%", max),
										cx.theme().danger,
									),
									(
										"Average",
										format!("{:.0}%", avg),
										cx.theme().primary,
									),
									(
										"Temperature",
										temp_str,
										cx.theme().warning,
									),
								],
							)),
					),
			)
		})
		.child(
			card("Per-Core Usage", cx).child(
				div()
					.flex()
					.flex_row()
					.flex_wrap()
					.gap(px(10.0))
					.children(cores.iter().map(|c| core_tile(c, cx))),
			),
		)
		.child(
			card("Usage History (%)", cx).child(chart_box(
				LineChart::new(samples)
					.x(|s| s.label.clone())
					.y(|s| s.cpu)
					.stroke(history_color)
					.name("CPU (%)")
					.tick_margin(10)
					.id("cpu-history-chart"),
				"%",
				cx,
			)),
		)
}

fn load_color(v: f32, cx: &Context<PerformanceTab>) -> Hsla {
	if v > 80.0 {
		cx.theme().danger
	} else if v > 50.0 {
		cx.theme().warning
	} else {
		cx.theme().primary
	}
}

fn core_tile(
	core: &gpuitop_core::model::CpuCore,
	cx: &Context<PerformanceTab>,
) -> impl IntoElement {
	let color = load_color(core.usage_percent, cx);
	div()
		.w(px(92.0))
		.flex()
		.flex_col()
		.gap(px(4.0))
		.rounded(cx.theme().radius)
		.border_1()
		.border_color(cx.theme().border)
		.p(px(8.0))
		.child(
			div()
				.flex()
				.flex_row()
				.justify_between()
				.items_center()
				.child(
					div()
						.text_size(px(11.0))
						.text_color(cx.theme().muted_foreground)
						.child(format!("C{}", core.index)),
				)
				.child(
					div()
						.text_size(px(11.0))
						.font_weight(FontWeight::SEMIBOLD)
						.text_color(cx.theme().foreground)
						.child(format!("{:.0}%", core.usage_percent)),
				),
		)
		.child(
			Progress::new(format!("core-{}", core.index))
				.value(core.usage_percent)
				.color(color),
		)
		.when(core.frequency_mhz > 0, |el| {
			el.child(
				div()
					.text_size(px(10.0))
					.text_color(cx.theme().muted_foreground)
					.child(format!("{} MHz", core.frequency_mhz)),
			)
		})
}
