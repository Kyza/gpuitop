use crate::data::config::Config;
use crate::data::model::Theme;
use crate::data::settings;
use crate::ui::settings::SettingsTab;
use gpui::*;
use gpui_component::{
	setting::{SettingField, SettingGroup, SettingItem, SettingPage},
	Icon, IconName,
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

pub fn general_page(
	view: &Entity<SettingsTab>,
	refresh_ms: &Arc<AtomicU64>,
	theme_cell: &Rc<Cell<Theme>>,
	default_config: &Config,
) -> SettingPage {
	let view = view.clone();
	let refresh_ms = refresh_ms.clone();
	let theme_cell = theme_cell.clone();
	let default_refresh = SharedString::from(
		default_config.general.interface.refresh_ms.to_string(),
	);
	let default_theme = SharedString::from(
		default_config.general.interface.theme.to_string(),
	);

	SettingPage::new("General")
		.default_open(true)
		.icon(Icon::new(IconName::Settings2))
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
								view.read(cx)
									.config
									.general
									.interface
									.theme
									.to_string(),
							)
						}
					},
					{
						let view = view.clone();
						let theme_cell = theme_cell.clone();
						move |val: SharedString, cx: &mut App| {
							view.update(cx, |this, cx| {
								if let Some(new) = settings::set_theme(
									&mut this.config,
									&val,
								) {
									let mode = match new {
										Theme::Dark => {
											gpui_component::ThemeMode::Dark
										}
										Theme::Light => {
											gpui_component::ThemeMode::Light
										}
										Theme::System => {
											gpui_component::ThemeMode::Light
										}
									};
									gpui_component::Theme::change(
										mode, None, cx,
									);
									theme_cell.set(new);
									this.save();
									cx.notify();
								}
							});
						}
					},
				)
				.default_value(default_theme),
			)
			.description(
				"Change the appearance theme. System follows your desktop \
				 setting.",
			)
			.keywords(["appearance", "mode", "dark", "light"]),
		])])
}
