use crate::data::config::Config;
use crate::data::model::{GpuBackend, SystemSnapshot};
use crate::data::platform::system::InitSystem;
use crate::data::platform::{detect_gpu, detect_init, SystemCollector};
use crate::ui::assets::lucide::LucideIcon;
use crate::ui::performance::PerformanceTab;
use crate::ui::processes::ProcessesTab;
use crate::ui::settings::SettingsTab;
use gpui::prelude::*;
use gpui::App as GpuiApp;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::{DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_component::{ActiveTheme, Sizable, StyledExt, TitleBar};
use std::cell::RefCell;
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
				cx,
			)
		});

		let (tx, rx) = mpsc::channel::<SystemSnapshot>();
		let thread_refresh = refresh_ms.clone();

		std::thread::spawn(move || {
			let mut collector = SystemCollector::new(gpu_backend);
			loop {
				let snapshot = collector.tick();
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
					crate::data::themes::unpack_builtins_to_disk();

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

					crate::data::themes::load_builtins_into_registry(cx);

					let still_missing =
						!gpui_component::theme::ThemeRegistry::global(cx)
							.themes()
							.contains_key(&active_name);
					if still_missing {
						crate::data::theme::apply_theme_by_name(
							&SharedString::from("Default Dark"),
							None,
							cx,
						);
						this.config.general.interface.theme =
							SharedString::from("Default Dark");
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

fn theme_preview_item(
	menu: PopupMenu,
	name: &SharedString,
	mode: gpui_component::ThemeMode,
	current: &SharedString,
	config: &Config,
	preview: &Rc<RefCell<Option<SharedString>>>,
	on_commit: &Rc<dyn Fn(&SharedString, &mut GpuiApp)>,
) -> PopupMenu {
	let name_owned = name.clone();
	let item_name = name_owned.clone();
	let item_current = name_owned == *current;
	let item_config = RefCell::new(config.clone());
	let preview_name = name_owned.clone();
	let hover_preview = preview.clone();
	let click_preview = preview.clone();
	let on_commit = on_commit.clone();

	let item = PopupMenuItem::element({
		let name_display = name_owned.clone();
		move |_window: &mut Window, _cx: &mut GpuiApp| {
			let hover_preview = hover_preview.clone();
			let preview_name = preview_name.clone();
			div()
				.id(ElementId::Name(format!("theme-{name_display}").into()))
				.w_full()
				.flex()
				.flex_row()
				.items_center()
				.gap(px(4.0))
				.child(if mode.is_dark() {
					LucideIcon::Moon.icon().size(px(12.0)).text_color(
						gpui_component::theme::Theme::global(_cx)
							.muted_foreground,
					)
				} else {
					LucideIcon::Sun.icon().size(px(12.0)).text_color(
						gpui_component::theme::Theme::global(_cx)
							.muted_foreground,
					)
				})
				.child(SharedString::from(name_display.as_ref()))
				.child(div().flex_grow(1.0))
				.on_hover(
					move |hovered: &bool,
					      window: &mut Window,
					      cx: &mut GpuiApp| {
						if *hovered {
							if hover_preview.borrow().is_none() {
								let original =
									gpui_component::theme::Theme::global(cx)
										.theme_name()
										.clone();
								*hover_preview.borrow_mut() = Some(original);
							}
							crate::data::theme::apply_theme_by_name(
								&preview_name,
								Some(window),
								cx,
							);
						} else {
							let original = hover_preview.borrow().clone();
							if let Some(original) = original {
								crate::data::theme::apply_theme_by_name(
									&original,
									Some(window),
									cx,
								);
								*hover_preview.borrow_mut() = None;
							}
						}
					},
				)
		}
	})
	.checked(item_current)
	.disabled(item_current)
	.on_click(
		move |_: &ClickEvent, window: &mut Window, cx: &mut GpuiApp| {
			*click_preview.borrow_mut() = None;
			item_config.borrow_mut().general.interface.theme =
				item_name.clone();
			let _ = item_config.borrow().save();
			(on_commit)(&item_name, cx);
			crate::data::theme::apply_theme_by_name(
				&item_name,
				Some(window),
				cx,
			);
		},
	);
	menu.item(item)
}

pub(crate) fn build_theme_menu(
	mut menu: PopupMenu,
	config: &Config,
	on_commit: &Rc<dyn Fn(&SharedString, &mut GpuiApp)>,
	window: &mut Window,
	cx: &mut Context<PopupMenu>,
) -> PopupMenu {
	let families = crate::data::theme::list_theme_families(cx);
	let current = gpui_component::theme::Theme::global(cx)
		.theme_name()
		.clone();
	let preview = Rc::new(RefCell::new(None::<SharedString>));

	for family in families {
		if family.variants.len() == 1 {
			let (name, mode) = &family.variants[0];
			menu = theme_preview_item(
				menu, name, *mode, &current, config, &preview, on_commit,
			);
		} else {
			let family_name = SharedString::from(family.name.as_ref());
			let family_has_current =
				family.variants.iter().any(|(n, _)| n == &current);
			let submenu = PopupMenu::build(window, cx, {
				let variants = family.variants.clone();
				let current = current.clone();
				let config = config.clone();
				let preview = preview.clone();
				let on_commit = on_commit.clone();
				move |mut menu, _, _| {
					for (name, mode) in &variants {
						menu = theme_preview_item(
							menu, name, *mode, &current, &config, &preview,
							&on_commit,
						);
					}
					menu
				}
			});
			menu = menu.item(
				PopupMenuItem::submenu(family_name, submenu)
					.checked(family_has_current),
			);
		}
	}

	menu
}

impl Render for App {
	fn render(
		&mut self,
		window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		hotpath::measure_block!("render", {
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
														name.clone();
													this.settings_tab.update(
														cx,
														|tab, cx| {
															tab.config
																.general
																.interface
																.theme =
																name.clone();
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
										build_theme_menu(
											menu, &config, &on_commit,
											window, cx,
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
		})
	}
}
