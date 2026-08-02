use crate::data::model::ProcessInfo;
use crate::ui::assets::lucide::LucideIcon;
use gpui::*;
use gpui_component::menu::PopupMenuItem;
use std::rc::Rc;

fn send_signal(pid: i32, signal: i32) {
	unsafe {
		libc::kill(pid, signal);
	}
}

fn menu_item(
	label: impl Into<SharedString>,
	icon: Option<LucideIcon>,
	handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> PopupMenuItem {
	PopupMenuItem::Item {
		icon: icon.map(|i| {
			gpui_component::Icon::empty()
				.path(LucideIcon::path(i))
				.into()
		}),
		label: label.into(),
		disabled: false,
		checked: false,
		is_link: false,
		action: None,
		handler: Some(Rc::new(handler)),
	}
}

pub fn build_process_menu(proc: &ProcessInfo) -> Vec<PopupMenuItem> {
	let pid = proc.pid;
	let is_stopped = proc.state == 'T' || proc.state == 't';

	let mut items = vec![
		menu_item(
			"End Process",
			Some(LucideIcon::CircleOff),
			move |_, _, _| send_signal(pid, libc::SIGTERM),
		),
		menu_item("Force Kill", Some(LucideIcon::CircleX), move |_, _, _| {
			send_signal(pid, libc::SIGKILL)
		}),
	];

	if is_stopped {
		items.push(menu_item(
			"Resume",
			Some(LucideIcon::Play),
			move |_, _, _| send_signal(pid, libc::SIGCONT),
		));
	} else {
		items.push(menu_item(
			"Pause",
			Some(LucideIcon::Pause),
			move |_, _, _| send_signal(pid, libc::SIGSTOP),
		));
	}

	items.push(PopupMenuItem::separator());
	items.push(PopupMenuItem::Label("Send Signal".into()));
	items.push(menu_item("SIGHUP  (1)", None, move |_, _, _| {
		send_signal(pid, libc::SIGHUP)
	}));
	items.push(menu_item("SIGINT  (2)", None, move |_, _, _| {
		send_signal(pid, libc::SIGINT)
	}));
	items.push(menu_item("SIGQUIT  (3)", None, move |_, _, _| {
		send_signal(pid, libc::SIGQUIT)
	}));
	items.push(menu_item("SIGTERM  (15)", None, move |_, _, _| {
		send_signal(pid, libc::SIGTERM)
	}));
	items.push(menu_item("SIGKILL  (9)", None, move |_, _, _| {
		send_signal(pid, libc::SIGKILL)
	}));
	items.push(PopupMenuItem::separator());
	items.push(menu_item("SIGSTOP  (19)", None, move |_, _, _| {
		send_signal(pid, libc::SIGSTOP)
	}));
	items.push(menu_item("SIGCONT  (18)", None, move |_, _, _| {
		send_signal(pid, libc::SIGCONT)
	}));
	items.push(PopupMenuItem::separator());
	items.push(menu_item("SIGUSR1  (10)", None, move |_, _, _| {
		send_signal(pid, libc::SIGUSR1)
	}));
	items.push(menu_item("SIGUSR2  (12)", None, move |_, _, _| {
		send_signal(pid, libc::SIGUSR2)
	}));
	items.push(PopupMenuItem::separator());
	items.push(menu_item(
		"Copy PID",
		Some(LucideIcon::Copy),
		move |_, _, cx| {
			cx.write_to_clipboard(ClipboardItem::new_string(pid.to_string()));
		},
	));
	items.push(PopupMenuItem::separator());
	items.push(PopupMenuItem::Label(SharedString::from(format!(
		"PID: {}  —  {}",
		proc.pid, proc.name
	))));
	items.push(menu_item(
		"Properties",
		Some(LucideIcon::Info),
		move |_, _, _| {},
	));

	items
}
