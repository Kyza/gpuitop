use bytesize::ByteSize;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	description_list::DescriptionItem,
	input::{Input, InputState},
	scroll::ScrollableElement,
	tab::{Tab, TabBar},
	ActiveTheme,
};
use gpuitop_components::assets::lucide::LucideIcon;
use gpuitop_components::SelectableText;

use super::PropertiesWindow;
use crate::ProcessProperties;

pub const TAB_LABELS: &[&str] =
	&["Overview", "Memory", "I/O", "Environment", "FDs", "Limits"];

pub fn body(
	properties: Option<&ProcessProperties>,
	dead: bool,
	degraded: bool,
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

	let tabs = TAB_LABELS
		.iter()
		.map(|label| Tab::new().label(*label))
		.collect::<Vec<_>>();

	let tab_bar = TabBar::new("prop-tabs")
		.underline()
		.selected_index(tab_index)
		.on_click({
			let entity = cx.entity();
			move |index, _window, cx| {
				entity.update(cx, |this, cx| {
					this.tab_index = *index;
					cx.notify();
				});
			}
		})
		.children(tabs);

	let content: gpui::AnyElement = match tab_index {
		0 => overview(props, cx).into_any_element(),
		1 => memory(props, cx).into_any_element(),
		2 => io(props, cx).into_any_element(),
		3 => {
			environ(props, cx, name_search, content_search).into_any_element()
		}
		4 => fds(props, cx).into_any_element(),
		5 => limits(props, cx).into_any_element(),
		_ => div().into_any_element(),
	};

	div()
		.flex_1()
		.flex()
		.flex_col()
		.child(div().w_full().px(px(12.0)).child(tab_bar))
		.when(tab_index == 3 && !props.environ.is_empty(), |el| {
			el.child(search_bars(name_state, content_state, cx))
		})
		.child(
			div().flex_1().min_h_0().child(
				div().size_full().overflow_y_scrollbar().child(
					div()
						.px(px(12.0))
						.py(px(8.0))
						.when(degraded, |el| {
							el.child(
								div()
									.mb(px(8.0))
									.px(px(8.0))
									.py(px(4.0))
									.bg(rgba(0xffaa0050))
									.rounded(px(4.0))
									.text_color(rgb(0xff_cc66))
									.text_size(px(13.0))
									.child(
										"The process data could not be read \
										 (permissions?). Showing the last \
										 snapshot; still updating.",
									),
							)
						})
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
										"This process has exited. Data \
										 below is from the last snapshot \
										 and is no longer updating.",
									),
							)
						})
						.child(content),
				),
			),
		)
		.into_any_element()
}

fn dl() -> gpui_component::description_list::DescriptionList {
	gpui_component::description_list::DescriptionList::new()
		.columns(2)
		.bordered(true)
}

fn selectable_text(label: &str, text: String, color: Hsla) -> AnyElement {
	SelectableText::new(SharedString::from(label), text)
		.text_color(color)
		.into_any_element()
}

fn item(
	label: &str,
	text: String,
	cx: &Context<PropertiesWindow>,
) -> DescriptionItem {
	DescriptionItem::new(label)
		.value(selectable_text(label, text, cx.theme().foreground))
		.span(1)
}

fn item_yellow(label: &str, text: String) -> DescriptionItem {
	DescriptionItem::new(label)
		.value(selectable_text(label, text, gpui::yellow()))
		.span(1)
}

fn opt_item(
	label: &str,
	text: Option<String>,
	cx: &Context<PropertiesWindow>,
) -> DescriptionItem {
	match text {
		Some(t) => item(label, t, cx),
		None => item_yellow(label, "—".to_string()),
	}
}

fn separator() -> DescriptionItem {
	DescriptionItem::Separator
}

fn overview(
	props: &ProcessProperties,
	cx: &Context<PropertiesWindow>,
) -> impl IntoElement {
	let cmd = super::helpers::quote_command(&props.command);

	let groups = props
		.groups
		.iter()
		.map(|g| g.to_string())
		.collect::<Vec<_>>()
		.join(", ");

	dl().children([
		item("PID", props.pid.to_string(), cx),
		item("PPID", props.ppid.to_string(), cx),
		item("Name", props.name.clone(), cx),
		item("Command", cmd, cx),
		item("State", props.state_label.clone(), cx),
		item("Threads", props.threads.to_string(), cx),
		item("User", props.user.clone(), cx),
		item("UID", props.uid.to_string(), cx),
		item("Groups", groups, cx),
		separator(),
		item("Priority", props.priority.to_string(), cx),
		item("Nice", props.nice.to_string(), cx),
		item("CPU Time", format!("{:.2} s", props.cpu_time_secs), cx),
		opt_item("Processor", props.processor.map(|p| p.to_string()), cx),
		item(
			"Vol CTX Switches",
			props
				.voluntary_ctxt_switches
				.map(|v| v.to_string())
				.unwrap_or_else(|| "—".to_string()),
			cx,
		),
		item(
			"Nonvol CTX Switches",
			props
				.nonvoluntary_ctxt_switches
				.map(|v| v.to_string())
				.unwrap_or_else(|| "—".to_string()),
			cx,
		),
		separator(),
		opt_item("Executable", props.exe_path.clone(), cx),
		opt_item("CWD", props.cwd.clone(), cx),
		opt_item("Root", props.root_path.clone(), cx),
		separator(),
		item(
			"cgroups",
			if props.cgroups.is_empty() {
				"—".to_string()
			} else {
				props.cgroups.join("\n")
			},
			cx,
		),
	])
	.into_any_element()
}

fn memory(
	props: &ProcessProperties,
	cx: &Context<PropertiesWindow>,
) -> impl IntoElement {
	let smaps = &props.smaps;
	let bytes = |v: u64| ByteSize::b(v).to_string();
	dl().children([
		opt_item("VM Size", props.vm_size.map(bytes), cx),
		opt_item("VM Peak", props.vm_peak.map(bytes), cx),
		opt_item("VM RSS", props.vm_rss.map(bytes), cx),
		opt_item("VM HWM", props.vm_hwm.map(bytes), cx),
		opt_item("VM Data", props.vm_data.map(bytes), cx),
		opt_item("VM Stack", props.vm_stk.map(bytes), cx),
		opt_item("VM Exe", props.vm_exe.map(bytes), cx),
		opt_item("VM Lib", props.vm_lib.map(bytes), cx),
		opt_item("VM Swap", props.vm_swap.map(bytes), cx),
		opt_item("VM Locked", props.vm_lck.map(bytes), cx),
		opt_item("VM Pinned", props.vm_pin.map(bytes), cx),
		separator(),
		opt_item("RSS Anon", props.rss_anon.map(bytes), cx),
		opt_item("RSS File", props.rss_file.map(bytes), cx),
		opt_item("RSS Shmem", props.rss_shmem.map(bytes), cx),
		separator(),
		opt_item("PSS", smaps.as_ref().and_then(|s| s.pss).map(bytes), cx),
		opt_item("USS", smaps.as_ref().and_then(|s| s.uss).map(bytes), cx),
		opt_item("Swap", smaps.as_ref().and_then(|s| s.swap).map(bytes), cx),
		opt_item(
			"Shared Clean",
			smaps.as_ref().and_then(|s| s.shared_clean).map(bytes),
			cx,
		),
		opt_item(
			"Shared Dirty",
			smaps.as_ref().and_then(|s| s.shared_dirty).map(bytes),
			cx,
		),
		opt_item(
			"Private Clean",
			smaps.as_ref().and_then(|s| s.private_clean).map(bytes),
			cx,
		),
		opt_item(
			"Private Dirty",
			smaps.as_ref().and_then(|s| s.private_dirty).map(bytes),
			cx,
		),
		opt_item(
			"Referenced",
			smaps.as_ref().and_then(|s| s.referenced).map(bytes),
			cx,
		),
		opt_item(
			"Anonymous",
			smaps.as_ref().and_then(|s| s.anonymous).map(bytes),
			cx,
		),
	])
	.into_any_element()
}

fn io(
	props: &ProcessProperties,
	cx: &Context<PropertiesWindow>,
) -> impl IntoElement {
	let bytes = |v: u64| ByteSize::b(v).to_string();
	dl().children([
		opt_item("Read Bytes", props.io_read_bytes.map(bytes), cx),
		opt_item("Write Bytes", props.io_write_bytes.map(bytes), cx),
		opt_item(
			"Cancelled Write Bytes",
			props.io_cancelled_write_bytes.map(bytes),
			cx,
		),
		separator(),
		opt_item(
			"Read Chars",
			props.io_read_chars.map(|v| v.to_string()),
			cx,
		),
		opt_item(
			"Write Chars",
			props.io_write_chars.map(|v| v.to_string()),
			cx,
		),
		separator(),
		opt_item(
			"Read Syscalls",
			props.io_read_syscalls.map(|v| v.to_string()),
			cx,
		),
		opt_item(
			"Write Syscalls",
			props.io_write_syscalls.map(|v| v.to_string()),
			cx,
		),
	])
	.into_any_element()
}

fn search_bars(
	name_state: Option<&Entity<InputState>>,
	content_state: Option<&Entity<InputState>>,
	cx: &Context<PropertiesWindow>,
) -> impl IntoElement {
	let search_icon = || {
		LucideIcon::Search
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
		.flex()
		.flex_row()
		.gap(px(8.0))
		.px(px(12.0))
		.pt(px(8.0))
		.pb(px(8.0))
		.child(search_bar("Name", name_state))
		.child(search_bar("Content", content_state))
}

fn environ(
	props: &ProcessProperties,
	cx: &Context<PropertiesWindow>,
	name_search: &str,
	content_search: &str,
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

	let vars = super::helpers::filter_env_vars(
		&props.environ,
		name_search,
		content_search,
	);

	if vars.is_empty() {
		return div()
			.flex_1()
			.flex()
			.items_center()
			.justify_center()
			.text_color(cx.theme().muted_foreground)
			.child("No matches.")
			.into_any_element();
	}

	let items: Vec<DescriptionItem> = vars
		.iter()
		.flat_map(|(k, v)| {
			let entry = if v.is_empty() {
				item_yellow(k, "(empty)".to_string())
			} else {
				item(k, v.to_string(), cx)
			};
			[entry, separator()]
		})
		.collect();

	dl().children(items).into_any_element()
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
			[item(&label, target.clone(), cx), separator()]
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
			[item(&limit.name, label, cx), separator()]
		})
		.collect();

	dl().children(items).into_any_element()
}
