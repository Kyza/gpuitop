#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use gpui::prelude::*;
use gpui::*;
use gpui_component::{input::InputState, ActiveTheme, TitleBar};
use std::sync::mpsc;
use std::time::Duration;

use crate::ProcessProperties;
pub struct PropertiesWindow {
	pid: i32,
	properties: Option<ProcessProperties>,
	dead: bool,
	tab_index: usize,
	environ_name_state: Option<Entity<InputState>>,
	environ_content_state: Option<Entity<InputState>>,
	rx: mpsc::Receiver<Option<ProcessProperties>>,
}

impl PropertiesWindow {
	pub fn new(
		pid: i32,
		_icon_name: Option<String>,
		cx: &mut Context<Self>,
	) -> Self {
		let (tx, rx) = mpsc::channel();
		// Wake channel: the collector thread signals the UI after each
		// snapshot so we re-render on new data instead of every frame.
		let (wake_tx, wake_rx) = async_channel::unbounded::<()>();
		std::thread::spawn(move || loop {
			let result = crate::collect(pid);
			let is_none = result.is_none();
			let _ = tx.send(result);
			let _ = wake_tx.try_send(());
			if is_none {
				break;
			}
			std::thread::sleep(Duration::from_millis(1500));
		});

		// Foreground task: on each wake, request a re-render. The entity
		// isn't registered until new() returns, so we use a weak handle and
		// detach the task (dropping a Task would cancel it).
		cx.spawn(async move |this: gpui::WeakEntity<Self>, cx| {
			while wake_rx.recv().await.is_ok() {
				if this.update(cx, |_, cx| cx.notify()).is_err() {
					break;
				}
			}
		})
		.detach();

		Self {
			pid,
			properties: None,
			dead: false,
			tab_index: 0,
			environ_name_state: None,
			environ_content_state: None,
			rx,
		}
	}
}

impl Render for PropertiesWindow {
	#[hotpath::measure]
	fn render(
		&mut self,
		window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		// No perpetual on_next_frame/notify loop: rendering is driven by
		// cx.notify() from the wake task (new properties) and user interaction.
		while let Ok(result) = self.rx.try_recv() {
			match result {
				Some(props) => {
					self.properties = Some(props);
					self.dead = false;
				}
				None => {
					self.dead = true;
				}
			}
		}

		if self.environ_name_state.is_none() {
			self.environ_name_state = Some(cx.new(|cx| {
				InputState::new(window, cx).placeholder("Filter by name…")
			}));
		}
		if self.environ_content_state.is_none() {
			self.environ_content_state = Some(cx.new(|cx| {
				InputState::new(window, cx).placeholder("Filter by content…")
			}));
		}

		let name_search = self
			.environ_name_state
			.as_ref()
			.map(|s| s.read(cx).value().to_string())
			.unwrap_or_default();

		let content_search = self
			.environ_content_state
			.as_ref()
			.map(|s| s.read(cx).value().to_string())
			.unwrap_or_default();

		let title = if self.dead {
			format!("PID {} — Properties (exited)", self.pid)
		} else {
			format!("PID {} — Properties", self.pid)
		};

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
						.child(title),
				),
			)
			.child(render_body(
				self.properties.as_ref(),
				self.dead,
				self.tab_index,
				&name_search,
				&content_search,
				self.environ_name_state.as_ref(),
				self.environ_content_state.as_ref(),
				cx,
			))
			.into_any_element()
	}
}

#[cfg(target_os = "linux")]
fn render_body(
	properties: Option<&ProcessProperties>,
	dead: bool,
	tab_index: usize,
	name_search: &str,
	content_search: &str,
	name_state: Option<&Entity<InputState>>,
	content_state: Option<&Entity<InputState>>,
	cx: &mut Context<PropertiesWindow>,
) -> impl IntoElement {
	linux::body(
		properties,
		dead,
		tab_index,
		name_search,
		content_search,
		name_state,
		content_state,
		cx,
	)
}

#[cfg(target_os = "windows")]
fn render_body(
	properties: Option<&ProcessProperties>,
	dead: bool,
	tab_index: usize,
	_environ_search: &str,
	_content_search: &str,
	_environ_search_state: Option<&Entity<InputState>>,
	_content_state: Option<&Entity<InputState>>,
	cx: &mut Context<PropertiesWindow>,
) -> impl IntoElement {
	windows::body(properties, dead, tab_index, cx)
}

#[cfg(target_os = "macos")]
fn render_body(
	properties: Option<&ProcessProperties>,
	dead: bool,
	tab_index: usize,
	_environ_search: &str,
	_content_search: &str,
	_environ_search_state: Option<&Entity<InputState>>,
	_content_state: Option<&Entity<InputState>>,
	cx: &mut Context<PropertiesWindow>,
) -> impl IntoElement {
	macos::body(properties, dead, tab_index, cx)
}
