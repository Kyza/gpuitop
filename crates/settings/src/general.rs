use crate::mutations as settings;
use crate::tab::SettingsTab;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	button::Button,
	kbd::Kbd,
	menu::DropdownMenu,
	setting::{
		NumberFieldOptions, SettingField, SettingGroup, SettingItem,
		SettingPage,
	},
	ActiveTheme, StyledExt,
};
use gpuitop_components::assets::lucide::LucideIcon;
use gpuitop_core::config::{Config, DEFAULT_PAUSE_KEY};
use std::rc::Rc;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

struct PauseKeyCapture {
	recording: bool,
	focus: FocusHandle,
}

pub fn general_page(
	view: &Entity<SettingsTab>,
	refresh_ms: &Arc<AtomicU64>,
	default_config: &Config,
) -> SettingPage {
	let view = view.clone();
	let refresh_ms = refresh_ms.clone();
	let default_refresh = SharedString::from(
		default_config.general.interface.refresh_ms.to_string(),
	);

	SettingPage::new("General")
		.default_open(true)
		.icon(LucideIcon::Settings2.icon())
		.groups(vec![SettingGroup::new().title("Interface").items(vec![
			SettingItem::new(
				"Refresh Interval",
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
								view.read(cx)
									.config
									.get()
									.general
									.interface
									.refresh_ms
									.to_string(),
							)
						}
					},
					{
						let view = view.clone();
						let refresh_ms = refresh_ms.clone();
						move |val: SharedString, cx: &mut App| {
							view.update(cx, |this, cx| {
								this.config.mutate(|c| {
									settings::set_refresh_ms(
										c,
										&refresh_ms,
										&val,
									);
								});
								cx.notify();
							});
						}
					},
				)
				.default_value(default_refresh),
			)
			.description(
				"How often system data and the process list refresh. Lower \
				 values update more often but use more CPU.",
			)
			.keywords(["polling", "update", "interval"]),
			SettingItem::new(
				"Pause Key",
				SettingField::render({
					let view = view.clone();
					move |options, window, cx| {
						let key = SharedString::from(format!(
							"pause-key-{}-{}-{}",
							options.page_ix,
							options.group_ix,
							options.item_ix
						));
						let state =
							window.use_keyed_state(key, cx, |_window, cx| {
								PauseKeyCapture {
									recording: false,
									focus: cx.focus_handle(),
								}
							});
						let recording = state.read(cx).recording;
						let current_key = view
							.read(cx)
							.config
							.get()
							.general
							.interface
							.pause_key
							.clone();
						let stroke = Keystroke::parse(&current_key)
							.unwrap_or_else(|_| {
								Keystroke::parse(DEFAULT_PAUSE_KEY)
									.expect("escape parses")
							});
						let handle = state.read(cx).focus.clone();
						let click_state = state.clone();
						let key_state = state.clone();
						let key_view = view.clone();
						div()
							.id("settings-pause-key")
							.track_focus(&handle)
							.cursor_pointer()
							.h_flex()
							.gap(px(8.0))
							.py_0p5()
							.px_1()
							.rounded(cx.theme().radius)
							.bg(cx.theme().tokens.button)
							.text_color(cx.theme().button_foreground)
							.border_1()
							.border_color(cx.theme().input)
							.hover(|this| {
								this.bg(cx.theme().tokens.button_hover)
							})
							.when(recording, |this| {
								this.bg(cx.theme().tokens.button_active)
									.border_color(cx.theme().accent)
							})
							.when(recording, |this| {
								this.child(
									div()
										.text_sm()
										.text_color(
											cx.theme().button_foreground,
										)
										.child("Press keys\u{2026}"),
								)
							})
							.child(
								Kbd::new(stroke)
									.outline()
									.text_base()
									.px_1p5()
									.py_0p5(),
							)
							.on_mouse_down(
								MouseButton::Left,
								move |_e, window, cx| {
									window.focus(
										&click_state.read(cx).focus.clone(),
										cx,
									);
									click_state.update(cx, |s, _| {
										s.recording = !s.recording
									});
								},
							)
							.on_key_down(
								move |e: &KeyDownEvent, _window, cx| {
									if !key_state.read(cx).recording {
										return;
									}
									let combo = e.keystroke.unparse();
									if Keystroke::parse(&combo).is_err()
										|| matches!(
											e.keystroke.key.as_str(),
											"" | "control"
												| "ctrl" | "alt" | "shift"
												| "platform" | "function" | "cmd"
												| "win" | "super" | "fn"
												| "secondary"
										) {
										return;
									}
									key_view.update(cx, |this, cx| {
										this.config.mutate(|c| {
											settings::set_pause_key(
												c, &combo,
											);
										});
										cx.notify();
									});
									key_state.update(cx, |s, _| {
										s.recording = false
									});
									cx.stop_propagation();
								},
							)
							.into_any_element()
					}
				})
				.on_reset(
					{
						let view = view.clone();
						move |cx| {
							view.read(cx)
								.config
								.get()
								.general
								.interface
								.pause_key != DEFAULT_PAUSE_KEY
						}
					},
					{
						let view = view.clone();
						move |_window, cx| {
							view.update(cx, |this, cx| {
								this.config.mutate(|c| {
									settings::set_pause_key(
										c,
										DEFAULT_PAUSE_KEY,
									);
								});
								cx.notify();
							});
						}
					},
				),
			)
			.description(
				"Key combo that pauses and resumes collection. Click, then \
				 press the combo you want; click again to cancel.",
			)
			.keywords(["keybind", "shortcut", "pause"]),
			SettingItem::new(
				"Theme",
				SettingField::render({
					let view = view.clone();
					move |_options, _window, cx| {
						let config = view.read(cx).config.get().clone();
						let label = SharedString::from(
							config.general.interface.theme.as_str(),
						);
						let view = view.clone();
						Button::new("settings-theme-btn")
							.label(label)
							.dropdown_menu(move |menu, window, cx| {
								let on_commit: Rc<
									dyn Fn(&SharedString, &mut gpui::App),
								> = {
									let view = view.clone();
									Rc::new(move |name, cx| {
										view.update(cx, |this, cx| {
											this.config.mutate(|c| {
												c.general.interface.theme =
													name.as_ref().to_string();
											});
											cx.notify();
										});
									})
								};
								gpuitop_components::theme_menu::build_theme_menu(
									menu, &on_commit, window, cx,
								)
							})
					}
				}),
			)
			.description(
				"Select a theme from built-in or custom themes. Drop JSON \
				 theme files into ~/.config/gpuitop/themes/ to add more.",
			)
			.keywords(["appearance", "mode", "dark", "light"]),
			SettingItem::new(
				"Window Width",
				SettingField::number_input(
					NumberFieldOptions::default(),
					{
						let view = view.clone();
						move |cx: &App| {
							view.read(cx).config.get().window_size.0 as f64
						}
					},
					{
						let view = view.clone();
						move |val: f64, cx: &mut App| {
							view.update(cx, |this, cx| {
								this.config.mutate(|c| {
									settings::set_window_width(c, val);
								});
								cx.notify();
							});
						}
					},
				)
				.default_value(1100),
			)
			.keywords(["size", "width"]),
			SettingItem::new(
				"Window Height",
				SettingField::number_input(
					NumberFieldOptions::default(),
					{
						let view = view.clone();
						move |cx: &App| {
							view.read(cx).config.get().window_size.1 as f64
						}
					},
					{
						let view = view.clone();
						move |val: f64, cx: &mut App| {
							view.update(cx, |this, cx| {
								this.config.mutate(|c| {
									settings::set_window_height(c, val);
								});
								cx.notify();
							});
						}
					},
				)
				.default_value(700),
			)
			.keywords(["size", "height"]),
		])])
}
