use crate::config::Config;
use crate::model::{PidFilterMode, ResourceViewMode, VramPolling};
use crate::tabs::settings::SettingsTab;
use gpui::*;
use gpui_component::{
	setting::{SettingField, SettingGroup, SettingItem, SettingPage},
	Icon, IconName,
};

pub fn processes_page(
	view: &Entity<SettingsTab>,
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
