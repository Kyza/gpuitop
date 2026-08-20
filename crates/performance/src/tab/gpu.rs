use std::cmp::Reverse;

use bytesize::ByteSize;
use gpui::prelude::*;
use gpui::*;
use gpui_component::chart::LineChart;
use gpui_component::progress::ProgressCircle;
use gpui_component::{ActiveTheme, Sizable};
use gpuitop_components::assets::lucide::LucideIcon;
use gpuitop_components::theme::{tag_color, theme_dark_or_light, TagType};
use gpuitop_core::model::{
	GpuBackend, GpuDevice, ProcessSnapshot, SystemSnapshot,
};

use super::history::Sample;
use super::widgets::{card, chart_box, stat_tiles};
use super::PerformanceTab;

#[hotpath::measure]
pub fn gpu_tab(
	snapshot: &SystemSnapshot,
	samples: Vec<Sample>,
	cx: &mut Context<PerformanceTab>,
) -> impl IntoElement {
	let dark = theme_dark_or_light(cx);

	if snapshot.gpu_devices.is_empty() {
		let message = if snapshot.gpu_polling_enabled {
			"No GPU detected."
		} else {
			"GPU data is off."
		};
		return div().flex().flex_col().gap(px(14.0)).child(
			card("GPU", cx).child(
				div()
					.flex()
					.flex_col()
					.items_center()
					.gap(px(6.0))
					.py(px(24.0))
					.child(
						LucideIcon::Gpu
							.icon()
							.size(px(28.0))
							.text_color(cx.theme().muted_foreground),
					)
					.child(
						div()
							.text_size(px(13.0))
							.text_color(cx.theme().muted_foreground)
							.child(message),
					),
			),
		);
	}

	let mut top: Vec<&ProcessSnapshot> = snapshot
		.processes
		.iter()
		.filter(|p| p.vram.total() > 0)
		.collect();
	top.sort_by_key(|p| Reverse(p.vram.total()));
	top.truncate(10);

	let device_cards: Vec<AnyElement> = snapshot
		.gpu_devices
		.iter()
		.enumerate()
		.map(|(i, d)| device_card(d, i, dark, cx).into_any_element())
		.collect();

	div()
		.flex()
		.flex_col()
		.gap(px(14.0))
		.children(device_cards)
		.child(
			card("Utilization History (%)", cx).child(chart_box(
				LineChart::new(samples)
					.x(|s| s.label.clone())
					.y(|s| s.gpu_utilization)
					.stroke(cx.theme().chart_1)
					.name("GPU (%)")
					.tick_margin(10)
					.id("gpu-history-chart"),
				"%",
				cx,
			)),
		)
		.child(top_consumers(top, cx))
}

fn device_card(
	device: &GpuDevice,
	index: usize,
	dark: bool,
	cx: &mut Context<PerformanceTab>,
) -> impl IntoElement {
	let color = backend_color(device.backend, dark);
	let title = if device.name.is_empty() {
		format!("{} {}", device.backend, index + 1)
	} else {
		device.name.clone()
	};

	let vram_pct = if device.vram_total > 0 {
		device.vram_used as f32 / device.vram_total as f32 * 100.0
	} else {
		0.0
	};

	card(&title, cx).child(
		div()
			.flex()
			.flex_row()
			.flex_wrap()
			.items_center()
			.gap(px(24.0))
			.child(
				div()
					.flex()
					.flex_col()
					.items_center()
					.gap(px(4.0))
					.child(
						ProgressCircle::new(format!("gpu-vram-{index}"))
							.with_size(px(120.0))
							.value(vram_pct)
							.color(color)
							.child(
								div()
									.flex()
									.flex_col()
									.items_center()
									.child(
										div()
											.text_size(px(18.0))
											.font_weight(FontWeight::BOLD)
											.text_color(cx.theme().foreground)
											.child(
												if device.vram_total > 0 {
													format!(
														"{:.0}%",
														vram_pct
													)
												} else {
													"—".to_string()
												},
											),
									)
									.child(
										div()
											.text_size(px(10.0))
											.text_color(
												cx.theme().muted_foreground,
											)
											.child("VRAM"),
									),
							),
					)
					.child(
						div()
							.text_size(px(11.0))
							.text_color(cx.theme().muted_foreground)
							.child(format!(
								"{} / {}",
								ByteSize::b(device.vram_used),
								ByteSize::b(device.vram_total)
							)),
					),
			)
			.child(
				div().flex().flex_row().flex_wrap().gap(px(20.0)).children(
					stat_tiles(
						cx,
						&[
							(
								"Utilization",
								format!("{}%", device.utilization),
								color,
							),
							(
								"Temperature",
								temp_str(device.temperature),
								cx.theme().warning,
							),
							(
								"Power",
								power_str(device.power_watts),
								cx.theme().primary,
							),
							(
								"Core Clock",
								clock_str(device.core_clock_mhz),
								cx.theme().foreground,
							),
							(
								"Memory Clock",
								clock_str(device.memory_clock_mhz),
								cx.theme().foreground,
							),
							(
								"Fan",
								fan_str(device.fan_percent),
								cx.theme().foreground,
							),
						],
					),
				),
			),
	)
}

fn backend_color(backend: GpuBackend, dark: bool) -> Hsla {
	match backend {
		GpuBackend::Nvidia => tag_color(TagType::Nvidia, dark),
		GpuBackend::Amd => tag_color(TagType::Amd, dark),
		GpuBackend::None => tag_color(TagType::Vram, dark),
	}
}

fn temp_str(v: u32) -> String {
	if v > 0 {
		format!("{v} °C")
	} else {
		"—".into()
	}
}

fn power_str(v: f32) -> String {
	if v > 0.0 {
		format!("{v:.0} W")
	} else {
		"—".into()
	}
}

fn clock_str(v: u32) -> String {
	if v > 0 {
		format!("{v} MHz")
	} else {
		"—".into()
	}
}

fn fan_str(v: u32) -> String {
	if v > 0 {
		format!("{v}%")
	} else {
		"—".into()
	}
}

#[hotpath::measure]
fn top_consumers(
	top: Vec<&ProcessSnapshot>,
	cx: &Context<PerformanceTab>,
) -> impl IntoElement {
	if top.is_empty() {
		return card("Top VRAM Consumers", cx).child(
			div()
				.text_size(px(12.0))
				.text_color(cx.theme().muted_foreground)
				.child("No processes using VRAM."),
		);
	}

	card("Top VRAM Consumers", cx).child(
		div()
			.flex()
			.flex_col()
			.gap(px(6.0))
			.children(top.iter().map(|p| consumer_row(p, cx))),
	)
}

fn consumer_row(
	p: &ProcessSnapshot,
	cx: &Context<PerformanceTab>,
) -> impl IntoElement {
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
				.flex()
				.flex_row()
				.items_center()
				.gap(px(8.0))
				.child(
					div()
						.text_size(px(12.0))
						.text_color(cx.theme().foreground)
						.child(p.name.clone()),
				)
				.child(
					div()
						.text_size(px(11.0))
						.text_color(cx.theme().muted_foreground)
						.child(format!("PID {}", p.pid)),
				),
		)
		.child(
			div()
				.text_size(px(12.0))
				.font_weight(FontWeight::SEMIBOLD)
				.text_color(cx.theme().chart_5)
				.child(ByteSize::b(p.vram.total()).to_string()),
		)
}
