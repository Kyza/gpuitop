use crate::model::SystemSnapshot;
use bytesize::ByteSize;
use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme;
use std::rc::Rc;

pub struct PerformanceTab {
	snapshot: Rc<SystemSnapshot>,
}

impl PerformanceTab {
	pub fn new(
		snapshot: Rc<SystemSnapshot>,
		_cx: &mut Context<Self>,
	) -> Self {
		Self { snapshot }
	}

	pub fn set_snapshot(&mut self, snapshot: Rc<SystemSnapshot>) {
		self.snapshot = snapshot;
	}
}

impl Render for PerformanceTab {
	fn render(
		&mut self,
		_window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let cpu = &self.snapshot.cpu;
		let mem = &self.snapshot.memory;
		let disks = &self.snapshot.disks;
		let nets = &self.snapshot.networks;

		div()
			.size_full()
			.bg(cx.theme().background)
			.p(px(16.0))
			.flex()
			.flex_col()
			.gap(px(16.0))
			.child(section_title("CPU", cx))
			.child(progress_bar(
				cpu.overall_percent,
				format!("{:.1}%", cpu.overall_percent),
				cx,
			))
			.child(
				div()
					.flex()
					.flex_row()
					.flex_wrap()
					.gap(px(8.0))
					.children(cpu.cores.iter().map(|c| core_gauge(c, cx))),
			)
			.child(section_title("Memory", cx))
			.child(
				div()
					.flex()
					.flex_row()
					.gap(px(16.0))
					.child(bytes_label("Total", mem.total, cx))
					.child(bytes_label("Used", mem.used, cx))
					.child(bytes_label("Available", mem.available, cx))
					.child(bytes_label("Cached", mem.cached, cx)),
			)
			.child(progress_bar(
				if mem.total > 0 {
					mem.used as f32 / mem.total as f32 * 100.0
				} else {
					0.0
				},
				format!(
					"{} / {} ({:.1}%)",
					ByteSize::b(mem.used),
					ByteSize::b(mem.total),
					if mem.total > 0 {
						mem.used as f32 / mem.total as f32 * 100.0
					} else {
						0.0
					}
				),
				cx,
			))
			.child(bytes_label("Swap", mem.swap_used, cx))
			.child(section_title("Disk I/O", cx))
			.children(disks.iter().map(|d| disk_row(d, cx)))
			.child(section_title("Network", cx))
			.children(nets.iter().map(|n| net_row(n, cx)))
	}
}

fn core_gauge(
	core: &crate::model::CpuCore,
	cx: &mut Context<PerformanceTab>,
) -> impl IntoElement {
	let color = if core.usage_percent > 80.0 {
		cx.theme().danger
	} else if core.usage_percent > 50.0 {
		cx.theme().warning
	} else {
		cx.theme().primary
	};
	div()
		.w(px(60.0))
		.flex()
		.flex_col()
		.items_center()
		.gap(px(3.0))
		.child(
			div()
				.text_size(px(10.0))
				.text_color(cx.theme().muted_foreground)
				.child(format!("C{}", core.index)),
		)
		.child(
			div()
				.w_full()
				.h(px(8.0))
				.bg(cx.theme().muted.opacity(0.15))
				.rounded(px(2.0))
				.overflow_hidden()
				.child(
					div()
						.h_full()
						.bg(color)
						.w(relative(core.usage_percent / 100.0)),
				),
		)
		.child(
			div()
				.text_size(px(10.0))
				.text_color(cx.theme().foreground)
				.child(format!("{:.0}%", core.usage_percent)),
		)
}

fn disk_row(
	d: &crate::model::DiskInfo,
	cx: &mut Context<PerformanceTab>,
) -> impl IntoElement {
	let dev = d.device.clone();
	let r = format_rate(d.read_bytes_per_sec);
	let w = format_rate(d.write_bytes_per_sec);
	div()
		.w_full()
		.flex()
		.flex_row()
		.gap(px(16.0))
		.h(px(20.0))
		.items_center()
		.child(
			div()
				.text_size(px(12.0))
				.text_color(cx.theme().muted_foreground)
				.w(px(80.0))
				.child(dev),
		)
		.child(
			div()
				.text_size(px(12.0))
				.text_color(cx.theme().success)
				.child(format!("R: {r}")),
		)
		.child(
			div()
				.text_size(px(12.0))
				.text_color(cx.theme().danger)
				.child(format!("W: {w}")),
		)
}

fn net_row(
	n: &crate::model::NetInfo,
	cx: &mut Context<PerformanceTab>,
) -> impl IntoElement {
	let iface = n.interface.clone();
	let rx = format_rate(n.rx_bytes_per_sec);
	let tx = format_rate(n.tx_bytes_per_sec);
	div()
		.w_full()
		.flex()
		.flex_row()
		.gap(px(16.0))
		.h(px(20.0))
		.items_center()
		.child(
			div()
				.text_size(px(12.0))
				.text_color(cx.theme().muted_foreground)
				.w(px(80.0))
				.child(iface),
		)
		.child(
			div()
				.text_size(px(12.0))
				.text_color(cx.theme().success)
				.child(format!("▼ {rx}")),
		)
		.child(
			div()
				.text_size(px(12.0))
				.text_color(cx.theme().danger)
				.child(format!("▲ {tx}")),
		)
}

fn section_title(
	title: &str,
	cx: &mut Context<PerformanceTab>,
) -> impl IntoElement {
	let t = title.to_string();
	div()
		.text_size(px(14.0))
		.text_color(cx.theme().foreground)
		.font_weight(FontWeight::MEDIUM)
		.child(t)
}

fn bytes_label(
	label: &str,
	bytes: u64,
	cx: &mut Context<PerformanceTab>,
) -> impl IntoElement {
	let l = format!("{label}:");
	let b = ByteSize::b(bytes).to_string();
	div()
		.text_size(px(11.0))
		.flex()
		.flex_row()
		.gap(px(4.0))
		.child(div().text_color(cx.theme().muted_foreground).child(l))
		.child(div().text_color(cx.theme().foreground).child(b))
}

fn progress_bar(
	pct: f32,
	label: String,
	cx: &mut Context<PerformanceTab>,
) -> impl IntoElement {
	let clamped = pct.clamp(0.0, 100.0);
	div()
		.w_full()
		.h(px(28.0))
		.bg(cx.theme().muted.opacity(0.15))
		.rounded(px(4.0))
		.overflow_hidden()
		.child(
			div()
				.h_full()
				.bg(cx.theme().primary)
				.w(relative(clamped / 100.0))
				.flex()
				.items_center()
				.justify_end()
				.px(px(8.0))
				.text_size(px(11.0))
				.text_color(cx.theme().foreground)
				.when(clamped > 15.0, |d| d.child(label)),
		)
}

fn format_rate(bytes_per_sec: f64) -> String {
	if bytes_per_sec < 1024.0 {
		"0".into()
	} else {
		format!("{}/s", ByteSize::b(bytes_per_sec as u64))
	}
}
