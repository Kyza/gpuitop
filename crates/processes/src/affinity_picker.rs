use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::{ActiveTheme, Sizable};
use std::cell::{Cell, RefCell};

use crate::affinity;

pub struct AffinityPicker {
	pid: i32,
	name: String,
	cpus: RefCell<Vec<bool>>,
	paint: Cell<Option<bool>>,
}

impl AffinityPicker {
	pub fn new(pid: i32, name: String, _cx: &mut Context<Self>) -> Self {
		let count = affinity::cpu_count();
		let selected = affinity::allowed_cpus(pid)
			.map(|allowed| {
				let mut v = vec![false; count];
				for cpu in allowed {
					if cpu < count {
						v[cpu] = true;
					}
				}
				v
			})
			.unwrap_or_else(|_| vec![true; count]);
		Self {
			pid,
			name,
			cpus: RefCell::new(selected),
			paint: Cell::new(None),
		}
	}

	pub fn selected(&self) -> Vec<usize> {
		self.cpus
			.borrow()
			.iter()
			.enumerate()
			.filter(|(_, &on)| on)
			.map(|(i, _)| i)
			.collect()
	}

	fn set_cpu(&self, cpu: usize, on: bool) {
		if let Some(v) = self.cpus.borrow_mut().get_mut(cpu) {
			*v = on;
		}
	}

	fn set_all(&self, on: bool) {
		for v in self.cpus.borrow_mut().iter_mut() {
			*v = on;
		}
	}

	fn paint_mode(&self) -> Option<bool> {
		self.paint.get()
	}

	fn set_paint(&self, mode: Option<bool>) {
		self.paint.set(mode);
	}
}

impl Render for AffinityPicker {
	fn render(
		&mut self,
		_window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let count = affinity::cpu_count();
		let theme = cx.theme();
		let mut chips = Vec::new();
		for cpu in 0..count {
			let on = self.cpus.borrow()[cpu];
			chips.push(
				div()
					.id(("affinity-chip", cpu))
					.h(px(28.0))
					.min_w(px(32.0))
					.px(px(8.0))
					.flex()
					.items_center()
					.justify_center()
					.rounded(px(6.0))
					.cursor_pointer()
					.bg(if on { theme.primary } else { theme.muted })
					.text_color(if on {
						theme.primary_foreground
					} else {
						theme.muted_foreground
					})
					.text_size(px(12.0))
					.child(cpu.to_string())
					.on_mouse_down(
						MouseButton::Left,
						cx.listener(
							move |this: &mut Self,
							      _: &MouseDownEvent,
							      _,
							      cx| {
								let mode = !this.cpus.borrow()[cpu];
								this.set_paint(Some(mode));
								this.set_cpu(cpu, mode);
								cx.notify();
							},
						),
					)
					.on_mouse_move(cx.listener(
						move |this: &mut Self, _: &MouseMoveEvent, _, cx| {
							if let Some(mode) = this.paint_mode() {
								this.set_cpu(cpu, mode);
								cx.notify();
							}
						},
					))
					.on_mouse_up(
						MouseButton::Left,
						cx.listener(
							move |this: &mut Self,
							      _: &MouseUpEvent,
							      _,
							      cx| {
								this.set_paint(None);
								cx.notify();
							},
						),
					)
					.into_any_element(),
			);
		}

		div()
			.flex()
			.flex_col()
			.gap(px(12.0))
			.w_full()
			.child(
				div()
					.flex()
					.items_center()
					.justify_between()
					.child(
						div()
							.text_size(px(12.0))
							.text_color(theme.muted_foreground)
							.child(format!(
								"PID {} — {} · {} logical CPU{}",
								self.pid,
								self.name,
								count,
								if count == 1 { "" } else { "s" }
							)),
					)
					.child(
						div()
							.flex()
							.gap(px(8.0))
							.child(
								Button::new("affinity-all")
									.ghost()
									.compact()
									.small()
									.label("All CPUs")
									.on_click(cx.listener(
										|this: &mut Self, _, _, cx| {
											this.set_all(true);
											cx.notify();
										},
									)),
							)
							.child(
								Button::new("affinity-clear")
									.ghost()
									.compact()
									.small()
									.label("Clear CPUs")
									.on_click(cx.listener(
										|this: &mut Self, _, _, cx| {
											this.set_all(false);
											cx.notify();
										},
									)),
							),
					),
			)
			.child(div().flex().flex_wrap().gap(px(6.0)).children(chips))
			.child(
				div()
					.text_size(px(11.0))
					.text_color(theme.muted_foreground)
					.child(
						"Tip: click a chip to toggle it, or click and drag \
						 across chips to paint their state.",
					),
			)
	}
}
