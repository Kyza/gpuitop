use gpui::prelude::*;
use gpui::*;
use gpui_component::scroll::ScrollableElement;
use gpui_component::tab::{Tab, TabBar};
use gpui_component::ActiveTheme;
use gpuitop_components::assets::lucide::LucideIcon;
use gpuitop_core::model::SystemSnapshot;
use std::rc::Rc;

mod cpu;
mod disks;
mod gpu;
mod history;
mod memory;
mod network;
mod widgets;

use history::{History, Sample};

pub struct PerformanceTab {
	snapshot: Rc<SystemSnapshot>,
	active_tab: usize,
	history: History,
}

const TAB_LABELS: [&str; 5] = ["CPU", "Memory", "GPU", "Disks", "Network"];
const TAB_ICONS: [LucideIcon; 5] = [
	LucideIcon::Cpu,
	LucideIcon::MemoryStick,
	LucideIcon::Gpu,
	LucideIcon::HardDriveDownload,
	LucideIcon::Activity,
];

impl PerformanceTab {
	pub fn new(
		snapshot: Rc<SystemSnapshot>,
		active_tab: usize,
		_cx: &mut Context<Self>,
	) -> Self {
		Self {
			snapshot,
			active_tab: active_tab.min(TAB_LABELS.len().saturating_sub(1)),
			history: History::new(),
		}
	}

	pub fn set_snapshot(&mut self, snapshot: Rc<SystemSnapshot>) {
		let sample = Sample {
			label: String::new(),
			cpu: snapshot.cpu.overall_percent as f64,
			mem_used: snapshot.memory.used as f64,
			net_rx: snapshot
				.networks
				.iter()
				.map(|n| n.rx_bytes_per_sec)
				.sum(),
			net_tx: snapshot
				.networks
				.iter()
				.map(|n| n.tx_bytes_per_sec)
				.sum(),
			disk_read: snapshot
				.disks
				.iter()
				.map(|d| d.read_bytes_per_sec)
				.sum(),
			disk_write: snapshot
				.disks
				.iter()
				.map(|d| d.write_bytes_per_sec)
				.sum(),
			gpu_utilization: snapshot
				.gpu_devices
				.iter()
				.map(|d| d.utilization as f64)
				.fold(0.0, f64::max),
		};
		self.history.push(sample);
		self.snapshot = snapshot;
	}
}

impl Render for PerformanceTab {
	fn render(
		&mut self,
		_window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let tabs = TAB_LABELS
			.iter()
			.enumerate()
			.map(|(i, label)| {
				let fg = if i == self.active_tab {
					cx.theme().foreground
				} else {
					cx.theme().muted_foreground
				};
				Tab::new()
					.label(*label)
					.prefix(TAB_ICONS[i].icon().size(px(14.0)).text_color(fg))
			})
			.collect::<Vec<_>>();

		let samples = self.history.samples();
		let snapshot = self.snapshot.clone();

		let content = match self.active_tab {
			0 => cpu::cpu_tab(&snapshot.cpu, samples, cx).into_any_element(),
			1 => memory::memory_tab(&snapshot.memory, samples, cx)
				.into_any_element(),
			2 => gpu::gpu_tab(&snapshot, samples, cx).into_any_element(),
			3 => disks::disks_tab(&snapshot.disks, samples, cx)
				.into_any_element(),
			4 => network::network_tab(&snapshot.networks, samples, cx)
				.into_any_element(),
			_ => div().into_any_element(),
		};

		div()
			.size_full()
			.flex()
			.flex_col()
			.bg(cx.theme().background)
			.child(
				div().w_full().px(px(12.0)).child(
					TabBar::new("perf-tabs")
						.underline()
						.selected_index(self.active_tab)
						.on_click({
							let entity = cx.entity();
							move |index, _window, cx| {
								entity.update(cx, |this, cx| {
									this.active_tab = *index;
									cx.notify();
								});
							}
						})
						.children(tabs),
				),
			)
			.child(
				div()
					.flex_1()
					.min_h_0()
					.size_full()
					.overflow_y_scrollbar()
					.child(div().px(px(14.0)).py(px(14.0)).child(content)),
			)
	}
}
