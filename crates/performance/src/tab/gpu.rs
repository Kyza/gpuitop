use std::cell::Cell;
use std::cmp::Reverse;
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use bytesize::ByteSize;
use gpui::prelude::*;
use gpui::*;
use gpui_component::chart::{LineChart, PieChart};
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

const MAX_CONSUMERS: usize = 10;

#[derive(Clone)]
struct VramSlice {
	name: SharedString,
	bytes: f64,
	color: Hsla,
	pid: Option<i32>,
	index: usize,
}

#[hotpath::measure]
pub fn gpu_tab(
	snapshot: &SystemSnapshot,
	samples: Vec<Sample>,
	pin_request: Arc<AtomicI64>,
	pie_hover: Rc<Cell<Option<usize>>>,
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
	top.truncate(MAX_CONSUMERS);

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
		.child(vram_composition(snapshot, &top, pin_request, pie_hover, cx))
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
		elide(&device.name, 40)
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
fn vram_composition(
	snapshot: &SystemSnapshot,
	top: &[&ProcessSnapshot],
	pin_request: Arc<AtomicI64>,
	pie_hover: Rc<Cell<Option<usize>>>,
	cx: &mut Context<PerformanceTab>,
) -> impl IntoElement {
	let used_total: u64 =
		snapshot.gpu_devices.iter().map(|d| d.vram_used).sum();
	let total: u64 = snapshot.gpu_devices.iter().map(|d| d.vram_total).sum();
	let top_sum: u64 = top.iter().map(|p| p.vram.total()).sum();
	let other = used_total.saturating_sub(top_sum);
	let free = total.saturating_sub(used_total);

	let palette = slice_palette(cx);
	let mut slices: Vec<VramSlice> = top
		.iter()
		.enumerate()
		.map(|(i, p)| VramSlice {
			name: p.name.clone().into(),
			bytes: p.vram.total() as f64,
			color: palette[i % palette.len()],
			pid: Some(p.pid),
			index: i,
		})
		.collect();
	let mut next_index = slices.len();
	slices.push(VramSlice {
		name: "Other".into(),
		bytes: other as f64,
		color: cx.theme().muted_foreground,
		pid: None,
		index: next_index,
	});
	next_index += 1;
	if free > 0 {
		slices.push(VramSlice {
			name: "Free".into(),
			bytes: free as f64,
			color: cx.theme().border,
			pid: None,
			index: next_index,
		});
	}

	let hovered = pie_hover.get();
	card("VRAM Composition", cx).child(
		div()
			.flex()
			.flex_row()
			.items_center()
			.gap(px(24.0))
			.child(
				div().w(px(200.0)).h(px(200.0)).child(
					PieChart::new(slices.clone())
						.value(|d| d.bytes as f32)
						.color(move |d| {
							if Some(d.index) == hovered {
								brighten(d.color, 0.12)
							} else {
								d.color
							}
						})
						.inner_radius(55.0)
						.outer_radius_fn(move |arc| {
							if Some(arc.data.index) == hovered {
								92.0
							} else {
								80.0
							}
						}),
				),
			)
			.child(
				div()
					.flex_1()
					.min_w_0()
					.flex()
					.flex_col()
					.gap(px(6.0))
					.children(slices.iter().map(|s| {
						slice_row(
							s,
							pin_request.clone(),
							pie_hover.clone(),
							cx,
						)
					})),
			),
	)
}

fn slice_palette(cx: &Context<PerformanceTab>) -> Vec<Hsla> {
	let t = cx.theme();
	vec![
		t.chart_1, t.chart_2, t.chart_3, t.chart_4, t.chart_5, t.success,
		t.warning, t.danger,
	]
}

fn elide(s: &str, max: usize) -> String {
	if s.chars().count() <= max {
		s.to_string()
	} else {
		let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
		out.push('…');
		out
	}
}

fn brighten(c: Hsla, amount: f32) -> Hsla {
	Hsla {
		h: c.h,
		s: c.s,
		l: (c.l + amount).min(1.0),
		a: c.a,
	}
}

fn slice_row(
	slice: &VramSlice,
	pin_request: Arc<AtomicI64>,
	pie_hover: Rc<Cell<Option<usize>>>,
	cx: &mut Context<PerformanceTab>,
) -> AnyElement {
	let hover = pie_hover.clone();
	let idx = slice.index;
	let mut row = div()
		.id(ElementId::Name(
			format!("vram-slice-{}", slice.index).into(),
		))
		.flex()
		.flex_row()
		.items_center()
		.gap(px(8.0))
		.rounded(px(4.0))
		.px(px(4.0))
		.hover(|this| this.bg(cx.theme().muted.opacity(0.15)))
		.on_hover(cx.listener(move |_, is_hovered: &bool, _, cx| {
			hover.set(if *is_hovered { Some(idx) } else { None });
			cx.notify();
		}))
		.child(
			div()
				.w(px(10.0))
				.h(px(10.0))
				.rounded(px(2.0))
				.bg(slice.color),
		)
		.child(
			div()
				.flex_1()
				.text_size(px(12.0))
				.text_color(cx.theme().muted_foreground)
				.truncate()
				.child(slice.name.clone()),
		)
		.child(
			div()
				.flex_shrink_0()
				.text_size(px(12.0))
				.font_weight(FontWeight::SEMIBOLD)
				.text_color(cx.theme().foreground)
				.child(ByteSize::b(slice.bytes as u64).to_string()),
		);
	if let Some(pid) = slice.pid {
		row =
			row.cursor_pointer()
				.on_click(cx.listener(move |_, _, _, cx| {
					pin_request.store(pid as i64, Ordering::SeqCst);
					cx.notify();
				}));
	}
	row.into_any_element()
}
