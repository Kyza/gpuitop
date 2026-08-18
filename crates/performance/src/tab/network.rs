use gpui::prelude::*;
use gpui::*;
use gpui_component::chart::AreaChart;
use gpui_component::ActiveTheme;

use gpuitop_core::model::NetInfo;

use super::history::Sample;
use super::widgets::{card, chart_box, format_rate, stat_tiles};
use super::PerformanceTab;

pub fn network_tab(
	nets: &[NetInfo],
	samples: Vec<Sample>,
	cx: &mut Context<PerformanceTab>,
) -> impl IntoElement {
	let total_rx: f64 = nets.iter().map(|n| n.rx_bytes_per_sec).sum();
	let total_tx: f64 = nets.iter().map(|n| n.tx_bytes_per_sec).sum();

	div()
		.flex()
		.flex_col()
		.gap(px(14.0))
		.child(card("Network", cx).child(
			div().flex().flex_row().flex_wrap().gap(px(20.0)).children(
				stat_tiles(
					cx,
					&[
						(
							"Download",
							format_rate(total_rx),
							cx.theme().success,
						),
						("Upload", format_rate(total_tx), cx.theme().danger),
						(
							"Interfaces",
							nets.len().to_string(),
							cx.theme().foreground,
						),
					],
				),
			),
		))
		.child(
			card("Traffic History (MiB/s)", cx).child(chart_box(
				AreaChart::new(samples)
					.x(|s| s.label.clone())
					.y(|s| s.net_rx / (1024.0 * 1024.0))
					.stroke(cx.theme().chart_1)
					.fill(cx.theme().chart_1.opacity(0.3))
					.name("Download (MiB/s)")
					.y(|s| s.net_tx / (1024.0 * 1024.0))
					.stroke(cx.theme().chart_2)
					.fill(cx.theme().chart_2.opacity(0.3))
					.name("Upload (MiB/s)")
					.tick_margin(10)
					.id("net-history-chart"),
				"MiB/s",
				cx,
			)),
		)
		.child(card("Interfaces", cx).child(iface_list(nets, cx)))
}

fn iface_list(
	nets: &[NetInfo],
	cx: &Context<PerformanceTab>,
) -> impl IntoElement {
	if nets.is_empty() {
		return div()
			.text_size(px(12.0))
			.text_color(cx.theme().muted_foreground)
			.child("No network interfaces.");
	}

	div()
		.flex()
		.flex_col()
		.gap(px(6.0))
		.children(nets.iter().map(|n| iface_row(n, cx)))
}

fn iface_row(n: &NetInfo, cx: &Context<PerformanceTab>) -> impl IntoElement {
	div()
		.flex()
		.flex_row()
		.items_center()
		.justify_between()
		.py(px(4.0))
		.border_b_1()
		.border_color(cx.theme().border.opacity(0.4))
		.child(
			div()
				.text_size(px(12.0))
				.text_color(cx.theme().foreground)
				.w(px(120.0))
				.child(n.interface.clone()),
		)
		.child(
			div()
				.flex()
				.flex_row()
				.gap(px(20.0))
				.child(
					div()
						.text_size(px(12.0))
						.text_color(cx.theme().success)
						.child(format!(
							"↓ {}",
							format_rate(n.rx_bytes_per_sec)
						)),
				)
				.child(
					div()
						.text_size(px(12.0))
						.text_color(cx.theme().danger)
						.child(format!(
							"↑ {}",
							format_rate(n.tx_bytes_per_sec)
						)),
				),
		)
}
