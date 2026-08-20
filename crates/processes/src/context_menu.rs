use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::PopupMenuItem;
use gpui_component::notification::{Notification, NotificationType};
use gpui_component::{Root, Sizable, WindowExt};
use gpuitop_components::assets::lucide::LucideIcon;
use gpuitop_core::model::ProcessSnapshot;
use gpuitop_properties::PropertiesWindow;
use std::rc::Rc;

use crate::affinity;
use crate::affinity_picker::AffinityPicker;

fn send_signal(pid: i32, signal: i32) {
	unsafe {
		libc::kill(pid, signal);
	}
}

fn push_error(window: &mut Window, cx: &mut App, message: &str) {
	Root::update(window, cx, |root, window, cx| {
		root.push_notification(
			Notification::new()
				.with_type(NotificationType::Error)
				.title("Set CPU Affinity")
				.message(message.to_owned())
				.autohide(true),
			window,
			cx,
		);
	});
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

pub fn build_process_menu(proc: &ProcessSnapshot) -> Vec<PopupMenuItem> {
	let pid = proc.pid;
	let is_stopped = proc.state == 'T' || proc.state == 't';
	let name = proc.name.clone();

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
	items.push(PopupMenuItem::Item {
		icon: Some(
			gpui_component::Icon::empty()
				.path(LucideIcon::path(LucideIcon::Cpu))
				.into(),
		),
		label: "Set CPU Affinity…".into(),
		disabled: !proc.is_owned_by_current_user,
		checked: false,
		is_link: false,
		action: None,
		handler: Some(Rc::new(move |_, window, cx| {
			let picker =
				cx.new(|cx| AffinityPicker::new(pid, name.clone(), cx));
			let title = SharedString::from(format!(
				"Set CPU Affinity — {} (PID {})",
				name, pid
			));
			let apply_name = name.clone();
			window.open_dialog(cx, move |dialog, _window, _cx| {
				let content_picker = picker.clone();
				let ok_picker = picker.clone();
				let apply_name = apply_name.clone();
				let apply = move |window: &mut Window, cx: &mut App| {
					let selected = ok_picker.read(cx).selected();
					match affinity::set_affinity(pid, &selected) {
						Ok(()) => {
							Root::update(window, cx, |root, window, cx| {
								root.push_notification(
									Notification::new()
										.with_type(NotificationType::Success)
										.title("Set CPU Affinity")
										.message(format!(
											"Applied to {apply_name} (PID \
											 {pid})"
										))
										.autohide(true),
									window,
									cx,
								);
							});
							true
						}
						Err(err) => {
							push_error(
								window,
								cx,
								&format!("Failed to set CPU affinity: {err}"),
							);
							false
						}
					}
				};
				let apply_ok = apply.clone();
				dialog
					.title(title.clone())
					.content(move |content, _window, _cx| {
						content.child(content_picker.clone())
					})
					.on_ok(move |_, window, cx| apply_ok(window, cx))
					.footer(
						div()
							.flex()
							.justify_end()
							.gap(px(8.0))
							.w_full()
							.child(
								Button::new("affinity-cancel")
									.ghost()
									.compact()
									.small()
									.label("Cancel")
									.on_click(move |_, window, cx| {
										window.close_dialog(cx);
									}),
							)
							.child(
								Button::new("affinity-apply")
									.primary()
									.compact()
									.small()
									.label("Apply")
									.on_click(move |_, window, cx| {
										if apply(window, cx) {
											window.close_dialog(cx);
										}
									}),
							),
					)
			});
		})),
	});
	items.push(PopupMenuItem::separator());
	items.push(PopupMenuItem::Label(SharedString::from(format!(
		"PID: {}  —  {}",
		proc.pid, proc.name
	))));
	items.push(menu_item("Properties", Some(LucideIcon::Info), {
		let pid = proc.pid;
		let icon_name = proc.icon_name.clone();
		move |_, _, cx| {
			let opts = WindowOptions {
				window_bounds: Some(WindowBounds::Windowed(Bounds {
					origin: point(px(200.0), px(200.0)),
					size: size(px(600.0), px(500.0)),
				})),
				window_min_size: Some(size(px(400.0), px(300.0))),
				titlebar: Some(TitlebarOptions {
					title: Some(SharedString::new(format!(
						"Properties — PID {pid}"
					))),
					appears_transparent: true,
					..Default::default()
				}),
				window_decorations: Some(WindowDecorations::Client),
				..Default::default()
			};
			let _ = cx.open_window(opts, {
				let icon = icon_name.clone();
				move |window, cx| {
					let view = cx.new(|cx| {
						PropertiesWindow::new(pid, icon.clone(), cx)
					});
					cx.new(|cx| gpui_component::Root::new(view, window, cx))
				}
			});
		}
	}));

	items
}
