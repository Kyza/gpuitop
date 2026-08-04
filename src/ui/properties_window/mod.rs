#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use gpui::prelude::*;
use gpui::*;
use gpui_component::{ActiveTheme, TitleBar};

pub struct PropertiesWindow {
	pid: i32,
}

impl PropertiesWindow {
	pub fn new(pid: i32, _icon_name: Option<String>) -> Self {
		Self { pid }
	}
}

impl Render for PropertiesWindow {
	fn render(
		&mut self,
		window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		cx.on_next_frame(window, |_, _, cx| cx.notify());

		let alive = crate::data::snapshot::pid_alive(self.pid);

		if !alive {
			return div()
				.size_full()
				.flex()
				.flex_col()
				.bg(cx.theme().background)
				.child(
					TitleBar::new().child(
						div()
							.flex()
							.flex_row()
							.items_center()
							.px(px(12.0))
							.child(format!("PID {} — Properties", self.pid)),
					),
				)
				.child(
					div()
						.flex_1()
						.flex()
						.items_center()
						.justify_center()
						.text_color(cx.theme().muted_foreground)
						.child(format!("PID {} no longer exists.", self.pid)),
				)
				.into_any_element();
		}

		div()
			.size_full()
			.flex()
			.flex_col()
			.bg(cx.theme().background)
			.text_color(cx.theme().foreground)
			.child(
				TitleBar::new().child(
					div()
						.flex()
						.flex_row()
						.items_center()
						.px(px(12.0))
						.child(format!("PID {} — Properties", self.pid)),
				),
			)
			.child(render_body(self.pid, cx))
			.into_any_element()
	}
}

#[cfg(target_os = "linux")]
fn render_body(
	pid: i32,
	_cx: &mut Context<PropertiesWindow>,
) -> impl IntoElement {
	linux::body(pid)
}

#[cfg(target_os = "windows")]
fn render_body(
	pid: i32,
	_cx: &mut Context<PropertiesWindow>,
) -> impl IntoElement {
	windows::body(pid)
}

#[cfg(target_os = "macos")]
fn render_body(
	pid: i32,
	_cx: &mut Context<PropertiesWindow>,
) -> impl IntoElement {
	macos::body(pid)
}
