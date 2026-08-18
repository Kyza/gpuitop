use gpui::prelude::*;
use gpui::*;
use gpui_component::chart::AreaChart;
use gpui_component::ActiveTheme;

use gpuitop_core::model::DiskInfo;

use super::history::Sample;
use super::widgets::{card, chart_box, format_rate, stat_tiles};
use super::PerformanceTab;

pub fn disks_tab(
	disks: &[DiskInfo],
	samples: Vec<Sample>,
	cx: &mut Context<PerformanceTab>,
) -> impl IntoElement {
	let total_read: f64 = disks.iter().map(|d| d.read_bytes_per_sec).sum();
	let total_write: f64 = disks.iter().map(|d| d.write_bytes_per_sec).sum();

	div()
		.flex()
		.flex_col()
		.gap(px(14.0))
		.child(card("Disk I/O", cx).child(
			div().flex().flex_row().flex_wrap().gap(px(20.0)).children(
				stat_tiles(
					cx,
					&[
						("Read", format_rate(total_read), cx.theme().success),
						(
							"Write",
							format_rate(total_write),
							cx.theme().danger,
						),
						(
							"Devices",
							disks.len().to_string(),
							cx.theme().foreground,
						),
					],
				),
			),
		))
		.child(
			card("Throughput History (MiB/s)", cx).child(chart_box(
				AreaChart::new(samples)
					.x(|s| s.label.clone())
					.y(|s| s.disk_read / (1024.0 * 1024.0))
					.stroke(cx.theme().chart_1)
					.fill(cx.theme().chart_1.opacity(0.3))
					.name("Read (MiB/s)")
					.y(|s| s.disk_write / (1024.0 * 1024.0))
					.stroke(cx.theme().chart_2)
					.fill(cx.theme().chart_2.opacity(0.3))
					.name("Write (MiB/s)")
					.tick_margin(10)
					.id("disk-history-chart"),
				"MiB/s",
				cx,
			)),
		)
		.child(card("Devices", cx).child(device_list(disks, cx)))
}

fn device_list(
	disks: &[DiskInfo],
	cx: &Context<PerformanceTab>,
) -> impl IntoElement {
	if disks.is_empty() {
		return div()
			.text_size(px(12.0))
			.text_color(cx.theme().muted_foreground)
			.child("No disk activity.");
	}

	div()
		.flex()
		.flex_col()
		.gap(px(6.0))
		.children(disks.iter().map(|d| disk_row(d, cx)))
}

fn disk_row(d: &DiskInfo, cx: &Context<PerformanceTab>) -> impl IntoElement {
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
				.child(d.device.clone()),
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
							"R {}",
							format_rate(d.read_bytes_per_sec)
						)),
				)
				.child(
					div()
						.text_size(px(12.0))
						.text_color(cx.theme().danger)
						.child(format!(
							"W {}",
							format_rate(d.write_bytes_per_sec)
						)),
				),
		)
}
