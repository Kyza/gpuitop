use bytesize::ByteSize;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	description_list::DescriptionItem,
	input::{Input, InputState},
	ActiveTheme,
};

use crate::data::properties::ProcessProperties;
use crate::ui::properties_window::PropertiesWindow;

pub const TAB_LABELS: &[&str] =
	&["Overview", "Memory", "I/O", "Environment", "FDs", "Limits"];

pub fn body(
	properties: Option<&ProcessProperties>,
	dead: bool,
	tab_index: usize,
	name_search: &str,
	content_search: &str,
	name_state: Option<&Entity<InputState>>,
	content_state: Option<&Entity<InputState>>,
	cx: &mut Context<PropertiesWindow>,
) -> impl IntoElement {
	let Some(props) = properties else {
		return div()
			.flex_1()
			.flex()
			.items_center()
			.justify_center()
			.text_color(cx.theme().muted_foreground)
			.child("No data available.")
			.into_any_element();
	};

	let tab_bar = TAB_LABELS
		.iter()
		.enumerate()
		.map(|(i, label)| {
			let is_active = i == tab_index;
			let border = if is_active {
				cx.theme().primary
			} else {
				Hsla::default()
			};
			let fg = if is_active {
				cx.theme().foreground
			} else {
				cx.theme().muted_foreground
			};
			div()
				.id(ElementId::Name(format!("prop-tab-{i}").into()))
				.px(px(12.0))
				.py(px(6.0))
				.cursor(CursorStyle::PointingHand)
				.text_size(px(13.0))
				.text_color(fg)
				.border_b_2()
				.border_color(border)
				.child(*label)
				.on_click(cx.listener(move |this, _, _, cx| {
					this.tab_index = i;
					cx.notify();
				}))
				.into_any_element()
		})
		.collect::<Vec<_>>();

	let content: gpui::AnyElement = match tab_index {
		0 => overview(props, cx).into_any_element(),
		1 => memory(props, cx).into_any_element(),
		2 => io(props, cx).into_any_element(),
		3 => environ(
			props,
			cx,
			name_search,
			content_search,
			name_state,
			content_state,
		)
		.into_any_element(),
		4 => fds(props, cx).into_any_element(),
		5 => limits(props, cx).into_any_element(),
		_ => div().into_any_element(),
	};

	div()
		.flex_1()
		.flex()
		.flex_col()
		.child(
			div()
				.flex()
				.flex_row()
				.gap(px(2.0))
				.border_b_1()
				.border_color(cx.theme().border)
				.px(px(12.0))
				.children(tab_bar),
		)
		.child(
			div().flex_1().id("props-scroll").overflow_y_scroll().child(
				div()
					.px(px(12.0))
					.py(px(8.0))
					.when(dead, |el| {
						el.child(
							div()
								.mb(px(8.0))
								.px(px(8.0))
								.py(px(4.0))
								.bg(rgba(0xff000050))
								.rounded(px(4.0))
								.text_color(rgb(0xff_6666))
								.text_size(px(13.0))
								.child(
									"This process has exited. Data below is \
									 from the last snapshot and is no \
									 longer updating.",
								),
						)
					})
					.child(content),
			),
		)
		.into_any_element()
}

fn dl() -> gpui_component::description_list::DescriptionList {
	gpui_component::description_list::DescriptionList::new()
		.columns(2)
		.bordered(true)
}

fn item(label: &str, value: impl IntoElement + 'static) -> DescriptionItem {
	DescriptionItem::new(label)
		.value(value.into_any_element())
		.span(1)
}

fn opt_item<E: IntoElement + 'static>(
	label: &str,
	value: Option<E>,
) -> DescriptionItem {
	match value {
		Some(v) => item(label, v),
		None => item(
			label,
			div()
				.text_color(gpui::yellow())
				.child("—")
				.into_any_element(),
		),
	}
}

fn themed(
	text: impl ToString + 'static,
	cx: &Context<PropertiesWindow>,
) -> impl IntoElement {
	let s: String = text.to_string();
	div()
		.text_color(cx.theme().foreground)
		.child(s)
		.into_any_element()
}

fn light<E: IntoElement + 'static>(el: E) -> impl IntoElement {
	div()
		.text_color(gpui::yellow())
		.child(el)
		.into_any_element()
}

fn separator() -> DescriptionItem {
	DescriptionItem::Separator
}

fn overview(
	props: &ProcessProperties,
	cx: &Context<PropertiesWindow>,
) -> impl IntoElement {
	let cmd = props
		.command
		.iter()
		.map(|s| {
			if s.contains(char::is_whitespace) {
				format!("'{}'", s.replace('\'', "\\'"))
			} else {
				s.clone()
			}
		})
		.collect::<Vec<_>>()
		.join(" ");

	let groups = props
		.groups
		.iter()
		.map(|g| g.to_string())
		.collect::<Vec<_>>()
		.join(", ");

	dl().children([
		item("PID", themed(props.pid.to_string(), cx)),
		item("PPID", themed(props.ppid.to_string(), cx)),
		item("Name", themed(props.name.clone(), cx)),
		item("Command", themed(cmd, cx)),
		item("State", themed(props.state_label.clone(), cx)),
		item("Threads", themed(props.threads.to_string(), cx)),
		item("User", themed(props.user.clone(), cx)),
		item("UID", themed(props.uid.to_string(), cx)),
		item("Groups", themed(groups, cx)),
		separator(),
		item("Priority", themed(props.priority.to_string(), cx)),
		item("Nice", themed(props.nice.to_string(), cx)),
		item(
			"CPU Time",
			themed(format!("{:.2} s", props.cpu_time_secs), cx),
		),
		opt_item(
			"Processor",
			props.processor.map(|p| themed(p.to_string(), cx)),
		),
		item(
			"Vol CTX Switches",
			themed(
				props
					.voluntary_ctxt_switches
					.map(|v| v.to_string())
					.unwrap_or_else(|| "—".to_string()),
				cx,
			),
		),
		item(
			"Nonvol CTX Switches",
			themed(
				props
					.nonvoluntary_ctxt_switches
					.map(|v| v.to_string())
					.unwrap_or_else(|| "—".to_string()),
				cx,
			),
		),
		separator(),
		opt_item(
			"Executable",
			props.exe_path.as_ref().map(|p| themed(p.clone(), cx)),
		),
		opt_item("CWD", props.cwd.as_ref().map(|p| themed(p.clone(), cx))),
		opt_item(
			"Root",
			props.root_path.as_ref().map(|p| themed(p.clone(), cx)),
		),
		separator(),
		item(
			"cgroups",
			themed(
				if props.cgroups.is_empty() {
					"—".to_string()
				} else {
					props.cgroups.join("\n")
				},
				cx,
			),
		),
	])
	.into_any_element()
}

fn memory(
	props: &ProcessProperties,
	cx: &Context<PropertiesWindow>,
) -> impl IntoElement {
	let smaps = &props.smaps;
	dl().children([
		opt_item(
			"VM Size",
			props
				.vm_size
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"VM Peak",
			props
				.vm_peak
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"VM RSS",
			props.vm_rss.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"VM HWM",
			props.vm_hwm.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"VM Data",
			props
				.vm_data
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"VM Stack",
			props.vm_stk.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"VM Exe",
			props.vm_exe.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"VM Lib",
			props.vm_lib.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"VM Swap",
			props
				.vm_swap
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"VM Locked",
			props.vm_lck.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"VM Pinned",
			props.vm_pin.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		separator(),
		opt_item(
			"RSS Anon",
			props
				.rss_anon
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"RSS File",
			props
				.rss_file
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"RSS Shmem",
			props
				.rss_shmem
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		separator(),
		opt_item(
			"PSS",
			smaps
				.as_ref()
				.and_then(|s| s.pss)
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"USS",
			smaps
				.as_ref()
				.and_then(|s| s.uss)
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"Swap",
			smaps
				.as_ref()
				.and_then(|s| s.swap)
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"Shared Clean",
			smaps
				.as_ref()
				.and_then(|s| s.shared_clean)
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"Shared Dirty",
			smaps
				.as_ref()
				.and_then(|s| s.shared_dirty)
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"Private Clean",
			smaps
				.as_ref()
				.and_then(|s| s.private_clean)
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"Private Dirty",
			smaps
				.as_ref()
				.and_then(|s| s.private_dirty)
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"Referenced",
			smaps
				.as_ref()
				.and_then(|s| s.referenced)
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"Anonymous",
			smaps
				.as_ref()
				.and_then(|s| s.anonymous)
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
	])
	.into_any_element()
}

fn io(
	props: &ProcessProperties,
	cx: &Context<PropertiesWindow>,
) -> impl IntoElement {
	dl().children([
		opt_item(
			"Read Bytes",
			props
				.io_read_bytes
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"Write Bytes",
			props
				.io_write_bytes
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		opt_item(
			"Cancelled Write Bytes",
			props
				.io_cancelled_write_bytes
				.map(|v| themed(ByteSize::b(v).to_string(), cx)),
		),
		separator(),
		opt_item(
			"Read Chars",
			props.io_read_chars.map(|v| themed(v.to_string(), cx)),
		),
		opt_item(
			"Write Chars",
			props.io_write_chars.map(|v| themed(v.to_string(), cx)),
		),
		separator(),
		opt_item(
			"Read Syscalls",
			props.io_read_syscalls.map(|v| themed(v.to_string(), cx)),
		),
		opt_item(
			"Write Syscalls",
			props.io_write_syscalls.map(|v| themed(v.to_string(), cx)),
		),
	])
	.into_any_element()
}

fn environ(
	props: &ProcessProperties,
	cx: &Context<PropertiesWindow>,
	name_search: &str,
	content_search: &str,
	name_state: Option<&Entity<InputState>>,
	content_state: Option<&Entity<InputState>>,
) -> impl IntoElement {
	if props.environ.is_empty() {
		return div()
			.flex_1()
			.flex()
			.items_center()
			.justify_center()
			.text_color(cx.theme().muted_foreground)
			.child("No environment variables.")
			.into_any_element();
	}

	let name_lower = name_search.to_lowercase();
	let content_lower = content_search.to_lowercase();

	let mut vars: Vec<(&str, &str)> = props
		.environ
		.iter()
		.map(|(k, v)| (k.as_str(), v.as_str()))
		.filter(|(k, v)| {
			let name_match = name_search.is_empty()
				|| k.to_lowercase().contains(&name_lower);
			let content_match = content_search.is_empty()
				|| v.to_lowercase().contains(&content_lower);
			name_match && content_match
		})
		.collect();
	vars.sort_by(|a, b| a.0.cmp(b.0));

	let search_icon = || {
		crate::ui::assets::lucide::LucideIcon::Search
			.icon()
			.size(px(12.0))
			.text_color(cx.theme().muted_foreground)
	};

	let search_bar =
		|_label: &str, state: Option<&Entity<InputState>>| match state {
			Some(s) => Input::new(s)
				.w_full()
				.prefix(search_icon())
				.into_any_element(),
			None => div().into_any_element(),
		};

	div()
		.flex_1()
		.flex()
		.flex_col()
		.child(
			div()
				.flex()
				.flex_row()
				.gap(px(8.0))
				.px(px(12.0))
				.pt(px(8.0))
				.child(search_bar("Name", name_state))
				.child(search_bar("Content", content_state)),
		)
		.child({
			if vars.is_empty() {
				return div()
					.flex_1()
					.flex()
					.items_center()
					.justify_center()
					.text_color(cx.theme().muted_foreground)
					.child(
						if name_search.is_empty() && content_search.is_empty()
						{
							"No environment variables."
						} else {
							"No matches."
						},
					)
					.into_any_element();
			}

			let items: Vec<DescriptionItem> = vars
				.iter()
				.flat_map(|(k, v)| {
					let val: AnyElement = if v.is_empty() {
						light("(empty)").into_any_element()
					} else {
						themed(v.to_string(), cx).into_any_element()
					};
					[item(k, val), separator()]
				})
				.collect();

			dl().children(items).into_any_element()
		})
		.into_any_element()
}

fn fds(
	props: &ProcessProperties,
	cx: &Context<PropertiesWindow>,
) -> impl IntoElement {
	if props.fds.is_empty() {
		return div()
			.flex_1()
			.flex()
			.items_center()
			.justify_center()
			.text_color(cx.theme().muted_foreground)
			.child("No file descriptors or permission denied.")
			.into_any_element();
	}

	let items: Vec<DescriptionItem> = props
		.fds
		.iter()
		.enumerate()
		.flat_map(|(i, target)| {
			let label = format!("FD {}", i);
			[item(&label, themed(target.clone(), cx)), separator()]
		})
		.collect();

	dl().children(items).into_any_element()
}

fn limits(
	props: &ProcessProperties,
	cx: &Context<PropertiesWindow>,
) -> impl IntoElement {
	if props.limits.is_empty() {
		return div()
			.flex_1()
			.flex()
			.items_center()
			.justify_center()
			.text_color(cx.theme().muted_foreground)
			.child("No limit data available.")
			.into_any_element();
	}

	let items: Vec<DescriptionItem> = props
		.limits
		.iter()
		.flat_map(|limit| {
			let label = format!("soft: {}, hard: {}", limit.soft, limit.hard);
			[item(&limit.name, themed(label, cx)), separator()]
		})
		.collect();

	dl().children(items).into_any_element()
}
