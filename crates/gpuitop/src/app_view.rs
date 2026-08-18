use gpui::prelude::*;
use gpui::App as GpuiApp;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::DropdownMenu;
use gpui_component::tab::{Tab, TabBar};
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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

const PAUSE_KEY: &str = "escape";

pub struct App {
	active_tab: usize,
	pub config: Config,
	snapshot: Rc<SystemSnapshot>,
	gpu_backends: Vec<GpuBackend>,
	init_system: InitSystem,
	rx: mpsc::Receiver<SystemSnapshot>,
	paused: Arc<AtomicBool>,
	focus_handle: FocusHandle,
	processes_tab: Entity<ProcessesTab>,
	performance_tab: Entity<PerformanceTab>,
	settings_tab: Entity<SettingsTab>,
	_theme_observer: Subscription,
}

impl App {
	pub fn new(
		active_tab: usize,
		initial_settings_page: Option<usize>,
		performance_tab: Option<usize>,
		search: Option<String>,
		override_view: Option<bool>,
		config: Config,
		desktop_cache: Arc<DesktopEntryCache>,
		cx: &mut Context<Self>,
	) -> Self {
		let gpu_backends = detect_gpu();
		let init_system = detect_init();
		let focus_handle = cx.focus_handle();

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
		let performance_tab = cx.new(|cx| {
			PerformanceTab::new(
				initial_snapshot.clone(),
				performance_tab.unwrap_or(0),
				cx,
			)
		});
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
		let thread_backends = gpu_backends.clone();
		let paused = Arc::new(AtomicBool::new(false));
		let thread_paused = paused.clone();

		std::thread::spawn(move || {
			let mut state =
				CollectorState::new(thread_backends, desktop_cache);
			loop {
				if !thread_paused.load(Ordering::SeqCst) {
					let snapshot = collect_snapshot(&mut state);
					if tx.send(snapshot).is_err() {
						break;
					}
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
			gpu_backends,
			init_system,
			rx,
			paused,
			focus_handle,
			processes_tab,
			performance_tab,
			settings_tab,
			_theme_observer: theme_observer,
		}
	}
}

impl Focusable for App {
	fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
		self.focus_handle.clone()
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

		if window.focused(cx).is_none() {
			let handle = self.focus_handle.clone();
			window.focus(&handle, cx);
		}

		while let Ok(new_snap) = self.rx.try_recv() {
			if self.paused.load(Ordering::SeqCst) {
				continue;
			}
			self.gpu_backends = new_snap.gpu_backends.clone();
			let snap = Rc::new(new_snap);
			self.processes_tab
				.update(cx, |tab, _| tab.set_snapshot(snap.clone()));
			self.performance_tab
				.update(cx, |tab, _| tab.set_snapshot(snap.clone()));
			self.snapshot = snap;
		}

		let active = self.active_tab;
		let paused = self.paused.load(Ordering::SeqCst);
		let labels = ["Processes", "Performance", "Settings"];
		let icons = [
			LucideIcon::List,
			LucideIcon::ChartPie,
			LucideIcon::Settings2,
		];

		let gpu = self.gpu_backends.clone();
		let init = self.init_system;

		let tabs = labels
			.iter()
			.enumerate()
			.map(|(i, label)| {
				let is_active = i == active;
				let fg = if is_active {
					cx.theme().foreground
				} else {
					cx.theme().muted_foreground
				};
				Tab::new()
					.label(*label)
					.prefix(icons[i].icon().size(px(14.0)).text_color(fg))
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
			.relative()
			.bg(cx.theme().background)
			.track_focus(&self.focus_handle)
			.on_key_down(cx.listener(
				|this, e: &KeyDownEvent, _window, cx| {
					if e.keystroke.key == PAUSE_KEY {
						let p = !this.paused.load(Ordering::SeqCst);
						this.paused.store(p, Ordering::SeqCst);
						cx.notify();
					}
				},
			))
			.child(
				TitleBar::new().child(
					div()
						.flex()
						.flex_row()
						.items_center()
						.w_full()
						.child(
							TabBar::new("main-tabs")
								.underline()
								.selected_index(active)
								.on_click({
									let entity = cx.entity();
									move |index, window, cx| {
										entity.update(cx, |this, cx| {
											this.active_tab = *index;
											let handle =
												this.focus_handle.clone();
											window.focus(&handle, cx);
											if *index == 0 {
												this.processes_tab.update(
													cx,
													|tab, cx| {
														tab.focus_input(cx);
													},
												);
											}
											cx.notify();
										});
									}
								})
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
				let gpu_label = if gpu.is_empty() {
					"None".to_string()
				} else {
					gpu.iter()
						.map(|b| match b {
							GpuBackend::Nvidia => "NVIDIA",
							GpuBackend::Amd => "ROCm",
							GpuBackend::None => "None",
						})
						.collect::<Vec<_>>()
						.join(" + ")
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
					.child(div().flex_grow(1.0))
					.child(if paused { "Esc: Resume" } else { "Esc: Pause" })
			})
			.when(paused, |el| {
				el.child(
					div()
						.absolute()
						.top_0()
						.left_0()
						.right_0()
						.bottom_0()
						.flex()
						.items_center()
						.justify_center()
						.child(
							LucideIcon::Pause.icon().size_32().text_color(
								cx.theme().foreground.opacity(0.3),
							),
						),
				)
			})
	}
}
