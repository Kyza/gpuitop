use crate::config::Config;
use crate::model::*;
use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct SettingsTab {
	config: Config,
	refresh_ms: Arc<AtomicU64>,
}

impl SettingsTab {
	pub fn new(
		config: Config,
		refresh_ms: Arc<AtomicU64>,
		_cx: &mut Context<Self>,
	) -> Self {
		Self {
			config,
			refresh_ms,
		}
	}

	fn save(&self) {
		if let Err(e) = self.config.save() {
			eprintln!("Failed to save config: {e}");
		}
	}
}

impl Render for SettingsTab {
	fn render(
		&mut self,
		_window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		div()
			.size_full()
			.bg(cx.theme().background)
			.p(px(24.0))
			.flex()
			.flex_col()
			.gap(px(14.0))
			.id(ElementId::Name("settings-scroll".into()))
			.overflow_scroll()
			.child(section("General", cx))
			.child(self.render_refresh_setting(cx))
			.child(section("Processes", cx))
			.child(self.render_vram_setting(cx))
			.child(self.render_pid_filter_mode_setting(cx))
			.child(self.render_clear_search_setting(cx))
			.child(self.render_resource_view_setting(cx))
			.child(section("Columns", cx))
			.child(
				div()
					.text_size(px(11.0))
					.text_color(cx.theme().muted_foreground)
					.child(
						"Choose which columns appear in the process table. \
						 Name is always shown.",
					),
			)
			.child(self.render_column_toggles(cx))
			.child(section("About", cx))
			.child(
				div()
					.text_size(px(12.0))
					.text_color(cx.theme().muted_foreground)
					.child(
						"gpuitop v0.1.0 · Linux-first process manager · \
						 Built with GPUI",
					),
			)
	}
}

impl SettingsTab {
	fn render_refresh_setting(
		&self,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let intervals: Vec<(u64, &str)> = vec![
			(500, "0.5s"),
			(1000, "1.0s"),
			(1500, "1.5s"),
			(2000, "2.0s"),
			(3000, "3.0s"),
			(5000, "5.0s"),
		];
		setting_row(
			self,
			"Refresh interval",
			Some(
				"How often system data and the process list refresh. Lower \
				 values update more often but use more CPU.",
			),
			cx,
			move |this, cx| {
				intervals
					.iter()
					.map(|(ms, label)| {
						let is_selected = this.config.refresh_ms == *ms;
						let ms = *ms;
						toggle_button(
							label,
							is_selected,
							cx,
							move |this, cx| {
								this.config.refresh_ms = ms;
								this.refresh_ms
									.store(ms, Ordering::SeqCst);
								this.save();
								cx.notify();
							},
						)
					})
					.collect()
			},
		)
	}

	fn render_vram_setting(
		&self,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let options: Vec<(VramPolling, &str)> = vec![
			(VramPolling::Auto, "Auto"),
			(VramPolling::On, "On"),
			(VramPolling::Off, "Off"),
		];
		setting_row(
			self,
			"VRAM polling",
			Some(
				"Auto detects the GPU at startup and tracks VRAM if \
				 available. On forces polling. Off disables it completely.",
			),
			cx,
			move |this, cx| {
				options
					.iter()
					.map(|(v, label)| {
						let is_selected = this.config.vram_polling == *v;
						let v = *v;
						toggle_button(
							label,
							is_selected,
							cx,
							move |this, cx| {
								this.config.vram_polling = v;
								this.save();
								cx.notify();
							},
						)
					})
					.collect()
			},
		)
	}

	fn render_pid_filter_mode_setting(
		&self,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let options: Vec<(PidFilterMode, &str)> = vec![
			(PidFilterMode::DirectChildren, "Direct only"),
			(PidFilterMode::AllDescendants, "All descendants"),
		];
		setting_row(
			self,
			"Process tree filter",
			Some(
				"When filtering by a process, show only its direct children \
				 or all descendants.",
			),
			cx,
			move |this, cx| {
				options
					.iter()
					.map(|(m, label)| {
						let is_selected = this.config.pid_filter_mode == *m;
						let m = *m;
						toggle_button(
							label,
							is_selected,
							cx,
							move |this, cx| {
								this.config.pid_filter_mode = m;
								this.save();
								cx.notify();
							},
						)
					})
					.collect()
			},
		)
	}

	fn render_clear_search_setting(
		&self,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let options: Vec<(bool, &str)> = vec![(true, "Yes"), (false, "No")];
		setting_row(
			self,
			"Clear search on pin",
			Some(
				"When double-clicking a process to filter by it, also clear \
				 the search bar.",
			),
			cx,
			move |this, cx| {
				options
					.iter()
					.map(|(v, label)| {
						let is_selected =
							this.config.clear_search_on_pin == *v;
						let v = *v;
						toggle_button(
							label,
							is_selected,
							cx,
							move |this, cx| {
								this.config.clear_search_on_pin = v;
								this.save();
								cx.notify();
							},
						)
					})
					.collect()
			},
		)
	}

	fn render_resource_view_setting(
		&self,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let options: Vec<(ResourceViewMode, &str)> = vec![
			(ResourceViewMode::SelfOnly, "Self"),
			(ResourceViewMode::Cumulative, "Cumulative"),
		];
		setting_row(
			self,
			"Resource view",
			Some(
				"Self shows each process's own usage. Cumulative shows the \
				 process totalled with all its descendants.",
			),
			cx,
			move |this, cx| {
				options
					.iter()
					.map(|(m, label)| {
						let is_selected =
							this.config.resource_view_mode == *m;
						let m = *m;
						toggle_button(
							label,
							is_selected,
							cx,
							move |this, cx| {
								this.config.resource_view_mode = m;
								this.save();
								cx.notify();
							},
						)
					})
					.collect()
			},
		)
	}

	fn render_column_toggles(
		&self,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let columns: Vec<(&str, bool, bool)> = vec![
			("Name", self.config.columns.name, true),
			("Process ID", self.config.columns.pid, false),
			("User", self.config.columns.user, false),
			("State", self.config.columns.state, false),
			("CPU usage", self.config.columns.cpu, false),
			("Memory usage", self.config.columns.memory, false),
			("VRAM usage", self.config.columns.vram, false),
			("Disk read", self.config.columns.disk_read, false),
			("Disk write", self.config.columns.disk_write, false),
			("Full command", self.config.columns.command, false),
		];

		div().w_full().flex().flex_col().gap(px(2.0)).children(
			columns.into_iter().map(|(name, visible, locked)| {
				let name_owned = name.to_string();
				div()
					.w_full()
					.flex()
					.flex_row()
					.items_center()
					.justify_between()
					.py(px(4.0))
					.child(
						div()
							.text_size(px(12.0))
							.text_color(cx.theme().foreground)
							.child(name_owned.clone()),
					)
					.child(if locked {
						div()
							.text_size(px(10.0))
							.text_color(
								cx.theme().muted_foreground.opacity(0.5),
							)
							.child("required")
							.into_any_element()
					} else {
						let enabled = visible;
						let name_str = name_owned.clone();
						div()
							.id(ElementId::Name(
								format!("col-{name_str}").into(),
							))
							.cursor(CursorStyle::PointingHand)
							.child(
								div()
									.px(px(12.0))
									.py(px(4.0))
									.rounded(px(4.0))
									.text_size(px(11.0))
									.bg(if enabled {
										cx.theme().primary
									} else {
										cx.theme().muted.opacity(0.12)
									})
									.text_color(if enabled {
										cx.theme().foreground
									} else {
										cx.theme().muted_foreground
									})
									.child(if enabled {
										"ON"
									} else {
										"OFF"
									}),
							)
							.on_click(cx.listener(move |this, _, _, cx| {
								let cols = &mut this.config.columns;
								match name_str.as_str() {
									"Process ID" => cols.pid = !cols.pid,
									"User" => cols.user = !cols.user,
									"State" => cols.state = !cols.state,
									"CPU usage" => cols.cpu = !cols.cpu,
									"Memory usage" => {
										cols.memory = !cols.memory
									}
									"VRAM usage" => cols.vram = !cols.vram,
									"Disk read" => {
										cols.disk_read = !cols.disk_read
									}
									"Disk write" => {
										cols.disk_write = !cols.disk_write
									}
									"Full command" => {
										cols.command = !cols.command
									}
									_ => {}
								}
								this.save();
								cx.notify();
							}))
							.into_any_element()
					})
			}),
		)
	}
}

fn section(title: &str, cx: &mut Context<SettingsTab>) -> impl IntoElement {
	let t = title.to_string();
	div()
		.w_full()
		.pt(px(8.0))
		.pb(px(4.0))
		.border_b_1()
		.border_color(cx.theme().border)
		.child(
			div()
				.text_size(px(13.0))
				.font_weight(FontWeight::MEDIUM)
				.text_color(cx.theme().primary)
				.child(t),
		)
}

fn setting_row(
	this: &SettingsTab,
	label: &str,
	desc: Option<&str>,
	cx: &mut Context<SettingsTab>,
	buttons: impl FnOnce(
		&SettingsTab,
		&mut Context<SettingsTab>,
	) -> Vec<AnyElement>,
) -> impl IntoElement {
	let label_owned = label.to_string();
	let desc_owned = desc.map(|d| d.to_string());
	div()
		.w_full()
		.flex()
		.flex_col()
		.gap(px(6.0))
		.py(px(4.0))
		.child(
			div()
				.flex()
				.flex_col()
				.gap(px(2.0))
				.child(
					div()
						.text_size(px(12.0))
						.text_color(cx.theme().foreground)
						.child(label_owned),
				)
				.when(desc_owned.is_some(), |d| {
					d.child(
						div()
							.text_size(px(11.0))
							.text_color(cx.theme().muted_foreground)
							.child(desc_owned.unwrap()),
					)
				}),
		)
		.child(
			div()
				.flex()
				.flex_row()
				.flex_wrap()
				.gap(px(4.0))
				.children(buttons(this, cx)),
		)
}

fn toggle_button(
	label: &str,
	selected: bool,
	cx: &mut Context<SettingsTab>,
	on_click: impl Fn(&mut SettingsTab, &mut Context<SettingsTab>) + 'static,
) -> AnyElement {
	let label_str = label.to_string();
	div()
		.id(ElementId::Name(format!("btn-{label_str}").into()))
		.px(px(10.0))
		.py(px(5.0))
		.text_size(px(12.0))
		.rounded(px(4.0))
		.bg(if selected {
			cx.theme().primary
		} else {
			cx.theme().muted.opacity(0.12)
		})
		.text_color(if selected {
			cx.theme().foreground
		} else {
			cx.theme().muted_foreground
		})
		.cursor(CursorStyle::PointingHand)
		.child(label_str)
		.on_click(cx.listener(move |this, _, _, cx| on_click(this, cx)))
		.into_any_element()
}
