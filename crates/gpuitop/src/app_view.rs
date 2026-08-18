use gpui::prelude::*;
use gpui::App as GpuiApp;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::DropdownMenu;
use gpui_component::{ActiveTheme, Sizable, TitleBar};
use gpuitop_components::assets::lucide::LucideIcon;
use gpuitop_core::config::Config;
use gpuitop_core::model::{GpuBackend, SystemSnapshot};
use gpuitop_core::service_manager::{detect_init, InitSystem};
use gpuitop_gpu::detect_gpu;
use gpuitop_icons::DesktopEntryCache;
use gpuitop_performance::PerformanceTab;
use gpuitop_processes::ProcessesTab;
use gpuitop_settings::SettingsTab;
use gpuitop_snapshot::{collect_snapshot, CollectorState};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

pub struct App {
	active_tab: usize,
	pub config: Config,
	snapshot: Rc<SystemSnapshot>,
	gpu_backend: GpuBackend,
	init_system: InitSystem,
	rx: mpsc::Receiver<SystemSnapshot>,
	processes_tab: Entity<ProcessesTab>,
	performance_tab: Entity<PerformanceTab>,
	settings_tab: Entity<SettingsTab>,
	_theme_observer: Subscription,
}

impl App {
	pub fn new(
		active_tab: usize,
		initial_settings_page: Option<usize>,
		search: Option<String>,
		override_view: Option<bool>,
		config: Config,
		desktop_cache: Arc<DesktopEntryCache>,
		cx: &mut Context<Self>,
	) -> Self {
		let gpu_backend = detect_gpu();
		let init_system = detect_init();

		let initial_snapshot = Rc::new(SystemSnapshot::empty());

		let processes_tab = cx.new(|cx| {
			ProcessesTab::new(
				config.clone(),
				initial_snapshot.clone(),
				init_system,
				override_view,
				search,
				cx,
			)
		});
		let performance_tab =
			cx.new(|cx| PerformanceTab::new(initial_snapshot.clone(), cx));
		let refresh_ms =
			Arc::new(AtomicU64::new(config.general.interface.refresh_ms));
		let settings_tab = cx.new(|cx| {
			SettingsTab::new(
				config.clone(),
				refresh_ms.clone(),
				initial_settings_page,
				crate::built::DIRECT_DEPS,
				cx,
			)
		});

		let (tx, rx) = mpsc::channel::<SystemSnapshot>();
		let thread_refresh = refresh_ms.clone();

		std::thread::spawn(move || {
			let mut state = CollectorState::new(gpu_backend, desktop_cache);
			loop {
				let snapshot = collect_snapshot(&mut state);
				if tx.send(snapshot).is_err() {
					break;
				}
				let ms = thread_refresh.load(Ordering::SeqCst);
				std::thread::sleep(Duration::from_millis(ms));
			}
		});

		let theme_observer = cx
			.observe_global::<gpui_component::theme::ThemeRegistry>(
				|this, cx| {
					gpuitop_components::themes::unpack_builtins_to_disk();

					let active_name =
						gpui_component::theme::Theme::global(cx)
							.theme_name()
							.clone();
					let active_exists =
						gpui_component::theme::ThemeRegistry::global(cx)
							.themes()
							.contains_key(&active_name);
					if active_exists {
						return;
					}

					gpuitop_components::themes::load_builtins_into_registry(
						cx,
					);

					let still_missing =
						!gpui_component::theme::ThemeRegistry::global(cx)
							.themes()
							.contains_key(&active_name);
					if still_missing {
						gpuitop_components::themes::apply_theme_by_name(
							&SharedString::from("Default Dark"),
							None,
							cx,
						);
						this.config.general.interface.theme =
							"Default Dark".to_string();
						let _ = this.config.save();
						cx.notify();
					}
				},
			);

		Self {
			active_tab,
			config,
			snapshot: initial_snapshot,
			gpu_backend,
			init_system,
			rx,
			processes_tab,
			performance_tab,
			settings_tab,
			_theme_observer: theme_observer,
		}
	}
}

impl Render for App {
	#[hotpath::measure]
	fn render(
		&mut self,
		window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		cx.on_next_frame(window, |_, _, cx| cx.notify());

		while let Ok(new_snap) = self.rx.try_recv() {
			self.gpu_backend = new_snap.gpu_backend;
			let snap = Rc::new(new_snap);
			self.processes_tab
				.update(cx, |tab, _| tab.set_snapshot(snap.clone()));
			self.performance_tab
				.update(cx, |tab, _| tab.set_snapshot(snap.clone()));
			self.snapshot = snap;
		}

		let active = self.active_tab;
		let labels = ["Processes", "Performance", "Settings"];
		let icons = [
			LucideIcon::List,
			LucideIcon::ChartPie,
			LucideIcon::Settings2,
		];

		let gpu = self.gpu_backend;
		let init = self.init_system;

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
						icons[i]
							.icon()
							.w(px(14.0))
							.h(px(14.0))
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
						.child({
							let config = self.config.clone();
							let entity: Entity<App> = cx.entity().clone();
							Button::new("theme-btn")
								.ghost()
								.compact()
								.small()
								.icon(
									LucideIcon::Palette
										.icon()
										.size(px(14.0))
										.text_color(
											cx.theme().muted_foreground,
										),
								)
								.dropdown_menu(move |menu, window, cx| {
									let on_commit = {
										let entity = entity.clone();
										Rc::new(
											move |name: &SharedString,
											      cx: &mut GpuiApp| {
												entity.update(cx, |this, cx| {
												this.config
													.general
													.interface
													.theme =
													name.as_ref()
														.to_string();
													this.settings_tab.update(
														cx,
														|tab, cx| {
															tab.config
																.general
																.interface
																.theme =
																name.as_ref()
																	.to_string();
															cx.notify();
														},
													);
													cx.notify();
												});
											},
										) as Rc<
											dyn Fn(
												&SharedString,
												&mut GpuiApp,
											),
										>
									};
									gpuitop_components::theme_menu::build_theme_menu(
										menu, &config, &on_commit, window, cx,
									)
								})
						}),
				),
			)
			.child(div().flex_grow(1.0).size_full().child(content))
			.child({
				let gpu_label = match gpu {
					GpuBackend::Nvidia => "NVIDIA",
					GpuBackend::Amd => "ROCm",
					_ => "None",
				};
				div()
					.flex()
					.flex_row()
					.gap(px(12.0))
					.px(px(8.0))
					.py(px(2.0))
					.text_size(px(11.0))
					.text_color(cx.theme().muted_foreground)
					.border_t_1()
					.border_color(cx.theme().border)
					.child(format!("GPU: {gpu_label}"))
					.child(format!("Init: {init}"))
			})
	}
}
