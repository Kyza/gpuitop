use crate::collect::{detect_gpu, SystemCollector};
use crate::config::Config;
use crate::model::{GpuBackend, SystemSnapshot, Theme};
use crate::tabs::{PerformanceTab, ProcessesTab, SettingsTab};
use gpui::prelude::*;
use gpui::*;
use gpui_component::{ActiveTheme, Icon, IconName, TitleBar};
use parking_lot::RwLock;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

	pub struct App {
	active_tab: usize,
	refresh_ms: u64,
	config: Config,
	theme: Rc<Cell<Theme>>,
	snapshot: Arc<RwLock<SystemSnapshot>>,
	collecting: Arc<AtomicBool>,
	gpu_backend: Arc<RwLock<GpuBackend>>,
	processes_tab: Entity<ProcessesTab>,
	performance_tab: Entity<PerformanceTab>,
	settings_tab: Entity<SettingsTab>,
}

fn apply_theme(theme: Theme, window: &mut Window, app: &mut gpui::App) {
	let mode = match theme {
		Theme::Dark => gpui_component::ThemeMode::Dark,
		Theme::Light => gpui_component::ThemeMode::Light,
		Theme::System => window.appearance().into(),
	};
	gpui_component::Theme::change(mode, Some(window), app);
}

impl App {
	pub fn new(cx: &mut Context<Self>) -> Self {
		let config = Config::load();
		let snapshot = Arc::new(RwLock::new(SystemSnapshot::empty()));
		let gpu_backend = Arc::new(RwLock::new(detect_gpu()));
		let collecting = Arc::new(AtomicBool::new(false));
		let theme = Rc::new(Cell::new(config.theme));

		let processes_tab = cx.new(|cx| {
			ProcessesTab::new(config.clone(), snapshot.clone(), cx)
		});
		let performance_tab =
			cx.new(|cx| PerformanceTab::new(snapshot.clone(), cx));
		let settings_tab = cx.new(|cx| SettingsTab::new(config.clone(), cx));

		let refresh_ms = config.refresh_ms;
		let snap = snapshot.clone();
		let gpu = gpu_backend.clone();
		let col = collecting.clone();

		std::thread::spawn(move || {
			let mut collector = SystemCollector::new(snap, gpu);
			loop {
				col.store(true, Ordering::SeqCst);
				collector.tick();
				col.store(false, Ordering::SeqCst);
				std::thread::sleep(Duration::from_millis(refresh_ms));
			}
		});

		Self {
			active_tab: 0,
			refresh_ms: config.refresh_ms,
			config,
			theme,
			snapshot,
			collecting,
			gpu_backend,
			processes_tab,
			performance_tab,
			settings_tab,
		}
	}
}

impl Render for App {
	fn render(
		&mut self,
		window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		hotpath::measure_block!("render", {
			cx.on_next_frame(window, |_, _, cx| cx.notify());

			let active = self.active_tab;
			let labels = ["Processes", "Performance", "Settings"];
			let icons =
				[IconName::Cpu, IconName::ChartPie, IconName::Settings];

			let current_theme = self.theme.get();
			let theme_icon = match current_theme {
				Theme::Dark => IconName::Moon,
				Theme::Light => IconName::Sun,
				Theme::System => IconName::Palette,
			};
			let gpu = *self.gpu_backend.read();

			let tabs = labels
				.iter()
				.enumerate()
				.map(|(i, label)| {
					let is_active = i == active;
					let border = if is_active {
						cx.theme().primary
					} else {
						transparent_white()
					};
					let fg = if is_active {
						cx.theme().foreground
					} else {
						cx.theme().muted_foreground
					};
					div()
						.id(ElementId::Name(format!("tab-{i}").into()))
						.px(px(14.0))
						.h(px(32.0))
						.flex()
						.flex_row()
						.items_center()
						.gap(px(6.0))
						.cursor(CursorStyle::PointingHand)
						.text_size(px(13.0))
						.text_color(fg)
						.border_b_2()
						.border_color(border)
						.child(
							Icon::new(icons[i].clone())
								.size(px(14.0))
								.text_color(fg),
						)
						.child(*label)
						.on_click(cx.listener(move |this, _, _, cx| {
							this.active_tab = i;
							if i == 0 {
								this.processes_tab.update(cx, |tab, cx| {
									tab.focus_input(cx);
								});
							}
							cx.notify();
						}))
						.into_any_element()
				})
				.collect::<Vec<_>>();

			let content = match active {
				0 => self.processes_tab.clone().into_any_element(),
				1 => self.performance_tab.clone().into_any_element(),
				2 => self.settings_tab.clone().into_any_element(),
				_ => div().into_any_element(),
			};

			let now = Instant::now();
			let last_refresh = self.snapshot.read().timestamp;
		let refresh_progress =
			(now.duration_since(last_refresh).as_secs_f32()
				/ (self.refresh_ms as f32 / 1000.0))
				.clamp(0.0, 1.0);

			div()
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
							.w_full()
							.child(
								div()
									.flex()
									.flex_row()
									.items_center()
									.gap(px(2.0))
									.children(tabs),
							)
							.child(div().flex_grow(1.0))
							.when(gpu != GpuBackend::None, |el| {
								let label = match gpu {
									GpuBackend::Nvidia => "NVIDIA",
									GpuBackend::Amd => "ROCm",
									_ => "",
								};
								el.child(
									div()
										.px(px(8.0))
										.h(px(32.0))
										.flex()
										.items_center()
										.text_size(px(11.0))
										.text_color(
											cx.theme().muted_foreground,
										)
										.child(label),
								)
							})
							.child({
								let theme = self.theme.clone();
								let config = self.config.clone();
								div()
									.id(ElementId::Name("theme-btn".into()))
									// .tooltip(move |_, _| {
									// 	div()
									// 		.text_sm()
									// 		.child(theme_tooltip.clone())
									// })
									.px(px(8.0))
									.h(px(32.0))
									.flex()
									.items_center()
									.justify_center()
									.cursor(CursorStyle::PointingHand)
									.child(
										Icon::new(theme_icon)
											.size(px(14.0))
											.text_color(
												cx.theme().muted_foreground,
											),
									)
									.on_click(
										move |_,
										      window,
										      cx: &mut gpui::App| {
											let new = match theme.get() {
												Theme::Dark => Theme::Light,
												Theme::Light => Theme::System,
												Theme::System => Theme::Dark,
											};
											theme.set(new);
											apply_theme(new, window, cx);
											let mut config = config.clone();
											config.theme = new;
											let _ = config.save();
										},
									)
							}),
					),
				)
				.child(div().flex_grow(1.0).size_full().child(content))
			.child(
				div()
					.w_full()
					.h(px(3.0))
					.bg(cx.theme().muted.opacity(0.12))
					.child(
						div()
							.h_full()
							.bg(cx.theme().primary.opacity(0.5))
							.w(relative(refresh_progress)),
					),
			)
		})
	}
}
