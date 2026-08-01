use crate::config::{ColumnVisibility, Config};
use crate::model::*;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	ActiveTheme, Icon, IconName,
	setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings},
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct SettingsTab {
	config: Config,
	refresh_ms: Arc<AtomicU64>,
	theme: Rc<Cell<Theme>>,
}

impl SettingsTab {
	pub fn new(
		config: Config,
		refresh_ms: Arc<AtomicU64>,
		theme: Rc<Cell<Theme>>,
		_cx: &mut Context<Self>,
	) -> Self {
		Self {
			config,
			refresh_ms,
			theme,
		}
	}

	fn save(&self) {
		if let Err(e) = self.config.save() {
			eprintln!("Failed to save config: {e}");
		}
	}

	fn setting_pages(
		&self,
		_window: &mut Window,
		cx: &mut Context<Self>,
	) -> Vec<SettingPage> {
		let view = cx.entity();
		let refresh_ms = self.refresh_ms.clone();
		let theme_cell = self.theme.clone();
		let default_config = Config::default();

		vec![
			Self::general_page(&view, &refresh_ms, &theme_cell, &default_config),
			Self::processes_page(&view, &default_config),
			Self::columns_page(&view, &default_config),
			Self::about_page(),
		]
	}

	fn general_page(
		view: &Entity<Self>,
		refresh_ms: &Arc<AtomicU64>,
		theme_cell: &Rc<Cell<Theme>>,
		default_config: &Config,
	) -> SettingPage {
		let view = view.clone();
		let refresh_ms = refresh_ms.clone();
		let theme_cell = theme_cell.clone();
		let default_refresh = SharedString::from(default_config.refresh_ms.to_string());
		let default_theme = SharedString::from(default_config.theme.to_string());

		SettingPage::new("General")
			.default_open(true)
			.icon(Icon::new(IconName::Settings2))
			.groups(vec![SettingGroup::new().title("Interface").items(vec![
				SettingItem::new(
					"Refresh interval",
					SettingField::dropdown(
						vec![
							("500".into(), "0.5s".into()),
							("1000".into(), "1.0s".into()),
							("1500".into(), "1.5s".into()),
							("2000".into(), "2.0s".into()),
							("3000".into(), "3.0s".into()),
							("5000".into(), "5.0s".into()),
						],
						{
							let view = view.clone();
							move |cx: &App| {
								SharedString::from(
									view.read(cx).config.refresh_ms.to_string(),
								)
							}
						},
						{
							let view = view.clone();
							let refresh_ms = refresh_ms.clone();
							move |val: SharedString, cx: &mut App| {
								view.update(cx, |this, cx| {
									if let Ok(ms) = val.parse::<u64>() {
										this.config.refresh_ms = ms;
										refresh_ms.store(ms, Ordering::SeqCst);
										this.save();
										cx.notify();
									}
								});
							}
						},
					)
					.default_value(default_refresh),
				)
				.description(
					"How often system data and the process list refresh. \
					 Lower values update more often but use more CPU.",
				)
				.keywords(["polling", "update", "interval"]),
				SettingItem::new(
					"Theme",
					SettingField::dropdown(
						vec![
							("Dark".into(), "Dark".into()),
							("Light".into(), "Light".into()),
							("System".into(), "System".into()),
						],
						{
							let view = view.clone();
							move |cx: &App| {
								SharedString::from(
									view.read(cx).config.theme.to_string(),
								)
							}
						},
						{
							let view = view.clone();
							let theme_cell = theme_cell.clone();
							move |val: SharedString, cx: &mut App| {
								let new = match val.as_str() {
									"Dark" => Theme::Dark,
									"Light" => Theme::Light,
									"System" => Theme::System,
									_ => return,
								};
								let mode = match new {
									Theme::Dark => gpui_component::ThemeMode::Dark,
									Theme::Light => gpui_component::ThemeMode::Light,
									Theme::System => gpui_component::ThemeMode::Light,
								};
								gpui_component::Theme::change(mode, None, cx);
								theme_cell.set(new);
								view.update(cx, |this, cx| {
									this.config.theme = new;
									this.save();
									cx.notify();
								});
							}
						},
					)
					.default_value(default_theme),
				)
				.description("Change the appearance theme. System follows your desktop setting.")
				.keywords(["appearance", "mode", "dark", "light"]),
			])])
	}

	fn processes_page(
		view: &Entity<Self>,
		default_config: &Config,
	) -> SettingPage {
		let view = view.clone();
		let default_vram = SharedString::from(default_config.vram_polling.to_string());
		let default_filter = SharedString::from(default_config.pid_filter_mode.to_string());
		let default_resource = SharedString::from(default_config.resource_view_mode.to_string());
		let default_clear_search = default_config.clear_search_on_pin;

		SettingPage::new("Processes")
			.default_open(true)
			.icon(Icon::new(IconName::Cpu))
			.groups(vec![SettingGroup::new().title("Behaviour").items(vec![
				SettingItem::new(
					"VRAM polling",
					SettingField::dropdown(
						vec![
							("Auto".into(), "Auto".into()),
							("On".into(), "On".into()),
							("Off".into(), "Off".into()),
						],
						{
							let view = view.clone();
							move |cx: &App| {
								SharedString::from(
									view.read(cx).config.vram_polling.to_string(),
								)
							}
						},
						{
							let view = view.clone();
							move |val: SharedString, cx: &mut App| {
								view.update(cx, |this, cx| {
									this.config.vram_polling = match val.as_str() {
										"Auto" => VramPolling::Auto,
										"On" => VramPolling::On,
										_ => VramPolling::Off,
									};
									this.save();
									cx.notify();
								});
							}
						},
					)
					.default_value(default_vram),
				)
				.description(
					"Auto detects the GPU at startup and tracks VRAM if available. \
					 On forces polling. Off disables it completely.",
				)
				.keywords(["gpu", "memory", "video"]),
				SettingItem::new(
					"Process tree filter",
					SettingField::dropdown(
						vec![
							("Direct only".into(), "Direct only".into()),
							("All descendants".into(), "All descendants".into()),
						],
						{
							let view = view.clone();
							move |cx: &App| {
								SharedString::from(
									view.read(cx).config.pid_filter_mode.to_string(),
								)
							}
						},
						{
							let view = view.clone();
							move |val: SharedString, cx: &mut App| {
								view.update(cx, |this, cx| {
									this.config.pid_filter_mode = match val.as_str() {
										"Direct only" => PidFilterMode::DirectChildren,
										_ => PidFilterMode::AllDescendants,
									};
									this.save();
									cx.notify();
								});
							}
						},
					)
					.default_value(default_filter),
				)
				.description(
					"When filtering by a process, show only its direct children \
					 or all descendants.",
				)
				.keywords(["children", "pid", "tree"]),
				SettingItem::new(
					"Clear search on pin",
					SettingField::switch(
						{
							let view = view.clone();
							move |cx: &App| {
								view.read(cx).config.clear_search_on_pin
							}
						},
						{
							let view = view.clone();
							move |val: bool, cx: &mut App| {
								view.update(cx, |this, cx| {
									this.config.clear_search_on_pin = val;
									this.save();
									cx.notify();
								});
							}
						},
					)
					.default_value(default_clear_search),
				)
				.description(
					"When double-clicking a process to filter by it, also clear \
					 the search bar.",
				)
				.keywords(["double-click", "pin", "search"]),
				SettingItem::new(
					"Resource view",
					SettingField::dropdown(
						vec![
							("Self".into(), "Self".into()),
							("Cumulative".into(), "Cumulative".into()),
						],
						{
							let view = view.clone();
							move |cx: &App| {
								SharedString::from(
									view.read(cx).config.resource_view_mode.to_string(),
								)
							}
						},
						{
							let view = view.clone();
							move |val: SharedString, cx: &mut App| {
								view.update(cx, |this, cx| {
									this.config.resource_view_mode = match val.as_str() {
										"Self" => ResourceViewMode::SelfOnly,
										_ => ResourceViewMode::Cumulative,
									};
									this.save();
									cx.notify();
								});
							}
						},
					)
					.default_value(default_resource),
				)
				.description(
					"Self shows each process's own usage. Cumulative shows the \
					 process totalled with all its descendants.",
				)
				.keywords(["usage", "total", "cumulative"]),
			])])
	}

	fn columns_page(
		view: &Entity<Self>,
		default_config: &Config,
	) -> SettingPage {
		let view = view.clone();
		let default_cols = default_config.columns.clone();

		let column_items: Vec<(&str, &str, fn(&ColumnVisibility) -> bool, fn(&mut ColumnVisibility, bool), &[&str])> = vec![
			("Process ID", "The numeric process identifier.", |c| c.pid, |c, v| c.pid = v, &["pid", "id"]),
			("User", "The username that owns the process.", |c| c.user, |c, v| c.user = v, &["username", "owner"]),
			("State", "Process state (Running, Sleeping, Zombie, etc.).", |c| c.state, |c, v| c.state = v, &["status", "zombie"]),
			("CPU usage", "Percentage of CPU used by the process.", |c| c.cpu, |c, v| c.cpu = v, &["processor", "cpu_usage"]),
			("Memory usage", "Resident memory used by the process.", |c| c.memory, |c, v| c.memory = v, &["ram", "rss", "mem"]),
			("VRAM usage", "Dedicated GPU memory used by the process.", |c| c.vram, |c, v| c.vram = v, &["gpu", "video", "graphics"]),
			("Disk read", "Bytes read from disk.", |c| c.disk_read, |c, v| c.disk_read = v, &["io", "read_bytes"]),
			("Disk write", "Bytes written to disk.", |c| c.disk_write, |c, v| c.disk_write = v, &["io", "write_bytes"]),
			("Full command", "The complete command line of the process.", |c| c.command, |c, v| c.command = v, &["cmd", "cmdline", "args"]),
		];

		let items: Vec<SettingItem> = column_items
			.into_iter()
			.map(|(title, desc, getter, setter, keywords)| {
				let view = view.clone();
				let default_enabled = getter(&default_cols);
				let keywords: Vec<&str> = keywords.iter().copied().collect();
				SettingItem::new(
					title,
					SettingField::switch(
						{
							let view = view.clone();
							move |cx: &App| {
								getter(&view.read(cx).config.columns)
							}
						},
						{
							let view = view.clone();
							move |val: bool, cx: &mut App| {
								view.update(cx, |this, cx| {
									setter(&mut this.config.columns, val);
									this.save();
									cx.notify();
								});
							}
						},
					)
					.default_value(default_enabled),
				)
				.description(desc)
				.keywords(keywords)
			})
			.collect();

		SettingPage::new("Columns")
			.default_open(false)
			.icon(Icon::new(IconName::LayoutDashboard))
			.group(
				SettingGroup::new()
					.title("Process Table Columns")
					.items(items),
			)
	}

	fn about_page() -> SettingPage {
		SettingPage::new("About")
			.default_open(false)
			.icon(Icon::new(IconName::Info))
			.group(
				SettingGroup::new().item(SettingItem::render(|_options, _, cx| {
					gpui_component::v_flex()
						.gap_3()
						.w_full()
						.items_center()
						.justify_center()
						.child(
							Icon::new(IconName::Cpu)
								.size(px(32.0))
								.text_color(cx.theme().muted_foreground),
						)
						.child(
							gpui_component::label::Label::new("gpuitop v0.1.0")
								.text_lg(),
						)
						.child(
							gpui_component::label::Label::new(
								"Linux-first process manager",
							)
							.text_sm()
							.text_color(cx.theme().muted_foreground),
						)
						.child(
							gpui_component::label::Label::new(
								"Built with GPUI & gpui-component",
							)
							.text_sm()
							.text_color(cx.theme().muted_foreground),
						)
						.into_any()
				})),
			)
	}
}

impl Render for SettingsTab {
	fn render(
		&mut self,
		window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		Settings::new("gpuitop-settings").pages(self.setting_pages(window, cx))
	}
}
