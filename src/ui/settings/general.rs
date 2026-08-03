use crate::data::settings;
use crate::ui::settings::SettingsTab;
use crate::{data::config::Config, ui::assets::lucide::LucideIcon};
use gpui::*;
use gpui_component::{
	button::Button,
	menu::DropdownMenu,
	setting::{
		NumberFieldOptions, SettingField, SettingGroup, SettingItem,
		SettingPage,
	},
};
use std::rc::Rc;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

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
								settings::set_refresh_ms(
									&mut this.config,
									&refresh_ms,
									&val,
								);
								this.save();
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
				"Theme",
				SettingField::render({
					let view = view.clone();
					move |_options, _window, cx| {
						let config = view.read(cx).config.clone();
						let label = config.general.interface.theme.clone();
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
											this.config
												.general
												.interface
												.theme = name.clone();
											this.save();
											cx.notify();
										});
									})
								};
								crate::ui::app::app_view::build_theme_menu(
									menu, &config, &on_commit, window, cx,
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
							view.read(cx).config.window_size.0 as f64
						}
					},
					{
						let view = view.clone();
						move |val: f64, cx: &mut App| {
							view.update(cx, |this, cx| {
								settings::set_window_width(
									&mut this.config,
									val,
								);
								this.save();
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
							view.read(cx).config.window_size.1 as f64
						}
					},
					{
						let view = view.clone();
						move |val: f64, cx: &mut App| {
							view.update(cx, |this, cx| {
								settings::set_window_height(
									&mut this.config,
									val,
								);
								this.save();
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
