use crate::data::platform::process_details::{self, ProcessDetails};
use crate::data::platform::process_icon::resolve_icon_path;
use bytesize::ByteSize;
use gpui::prelude::*;
use gpui::*;
use gpui_component::scroll::ScrollableElement;
use gpui_component::ActiveTheme;

pub struct PropertiesWindow {
	details: ProcessDetails,
	icon_name: Option<String>,
}

impl PropertiesWindow {
	pub fn new(pid: i32, icon_name: Option<String>) -> Self {
		let details = process_details::collect(pid);
		Self { details, icon_name }
	}
}

impl Render for PropertiesWindow {
	fn render(
		&mut self,
		_window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let d = &self.details;

		if !d.exists {
			return div()
				.size_full()
				.flex()
				.items_center()
				.justify_center()
				.bg(cx.theme().background)
				.child("Process terminated")
				.into_any_element();
		}

		div()
			.size_full()
			.flex()
			.flex_col()
			.bg(cx.theme().background)
			.text_color(cx.theme().foreground)
			.child(header(d, &self.icon_name, cx))
			.child(
				div()
					.flex_1()
					.overflow_y_scrollbar()
					.px(px(16.0))
					.child(section("Overview", overview(d)))
					.child(section("Memory", memory(d)))
					.child(section("I/O", io(d)))
					.child(section("Limits", limits(d)))
					.child(section("Environment", environment(d)))
					.child(section("File Descriptors", file_descriptors(d))),
			)
			.into_any_element()
	}
}

fn header(
	d: &ProcessDetails,
	icon_name: &Option<String>,
	cx: &mut Context<PropertiesWindow>,
) -> impl IntoElement + use<> {
	let icon_path = icon_name.as_deref().and_then(resolve_icon_path);

	div()
		.flex()
		.flex_row()
		.items_center()
		.gap(px(12.0))
		.px(px(16.0))
		.py(px(12.0))
		.border_b_1()
		.border_color(cx.theme().border)
		.when_some(icon_path, |el, path| {
			el.child(
				div()
					.w(px(32.0))
					.h(px(32.0))
					.flex()
					.items_center()
					.justify_center()
					.child(img(path).object_fit(ObjectFit::Contain)),
			)
		})
		.child(
			div()
				.flex()
				.flex_col()
				.child(
					div()
						.text_lg()
						.font_weight(FontWeight::BOLD)
						.child(d.name.clone()),
				)
				.child(
					div()
						.text_sm()
						.text_color(cx.theme().muted_foreground)
						.child(format!(
							"PID: {}  —  State: {}",
							d.state,
							crate::data::model::state_label(d.state)
						)),
				),
		)
}

fn section(title: &str, body: impl IntoElement) -> impl IntoElement {
	let t = title.to_string();
	div()
		.flex()
		.flex_col()
		.gap(px(4.0))
		.py(px(10.0))
		.border_b_1()
		.border_color(gpui::transparent_black())
		.child(
			div()
				.text_sm()
				.font_weight(FontWeight::BOLD)
				.mb(px(4.0))
				.child(t),
		)
		.child(body)
}

fn row(label: &str, value: impl Into<SharedString>) -> impl IntoElement {
	let v = value.into();
	div()
		.flex()
		.flex_row()
		.gap(px(8.0))
		.py(px(1.0))
		.child(
			div()
				.w(px(160.0))
				.text_xs()
				.text_color(gpui::transparent_black())
				.child(label.to_string()),
		)
		.child(div().text_sm().child(v))
}

fn overview(d: &ProcessDetails) -> impl IntoElement + use<> {
	div()
		.flex()
		.flex_col()
		.child(row("Executable", d.exe_path.as_str()))
		.child(row("Working Directory", d.cwd_path.as_str()))
		.child(row("User", format!("{} (UID: {})", d.user_name, d.uid)))
		.child(row("Group", d.gid.to_string()))
		.child(row("Threads", d.threads.to_string()))
		.child(row("Parent PID", d.ppid.to_string()))
}

fn memory(d: &ProcessDetails) -> impl IntoElement + use<> {
	div()
		.flex()
		.flex_col()
		.child(row("VmRSS", ByteSize::b(d.vm_rss).to_string()))
		.child(row("VmSize", ByteSize::b(d.vm_size).to_string()))
		.child(row("VmPeak", ByteSize::b(d.vm_peak).to_string()))
		.child(row("VmData", ByteSize::b(d.vm_data).to_string()))
		.child(row("VmStk", ByteSize::b(d.vm_stk).to_string()))
		.child(row("VmExe", ByteSize::b(d.vm_exe).to_string()))
		.child(row("VmLib", ByteSize::b(d.vm_lib).to_string()))
		.child(row("VmSwap", ByteSize::b(d.vm_swap).to_string()))
}

fn io(d: &ProcessDetails) -> impl IntoElement + use<> {
	div()
		.flex()
		.flex_col()
		.child(row("Read", ByteSize::b(d.io_read_bytes).to_string()))
		.child(row("Write", ByteSize::b(d.io_write_bytes).to_string()))
		.child(row("Read Syscalls", d.io_read_syscalls.to_string()))
		.child(row("Write Syscalls", d.io_write_syscalls.to_string()))
}

fn limits(d: &ProcessDetails) -> impl IntoElement + use<> {
	let items = d
		.limits
		.iter()
		.map(|(name, val)| row(name, val.clone()))
		.collect::<Vec<_>>();
	div().flex().flex_col().children(items)
}

fn environment(d: &ProcessDetails) -> impl IntoElement + use<> {
	if d.environment.is_empty() {
		return div().text_xs().child("(unavailable)");
	}
	let items = d
		.environment
		.iter()
		.map(|(k, v)| {
			div()
				.flex()
				.flex_row()
				.gap(px(4.0))
				.text_xs()
				.child(div().font_weight(FontWeight::BOLD).child(k.clone()))
				.child(div().child(v.clone()))
		})
		.collect::<Vec<_>>();
	div().flex().flex_col().gap(px(1.0)).children(items)
}

fn file_descriptors(d: &ProcessDetails) -> impl IntoElement + use<> {
	if d.file_descriptors.is_empty() {
		return div().text_xs().child("(unavailable)");
	}
	let items = d
		.file_descriptors
		.iter()
		.map(|(fd, target)| {
			div()
				.flex()
				.flex_row()
				.gap(px(8.0))
				.text_xs()
				.child(
					div()
						.w(px(40.0))
						.font_weight(FontWeight::BOLD)
						.child(fd.clone()),
				)
				.child(div().child(target.clone()))
		})
		.collect::<Vec<_>>();
	div().flex().flex_col().gap(px(1.0)).children(items)
}
