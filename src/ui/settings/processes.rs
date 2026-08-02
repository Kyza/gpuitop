use crate::data::config::Config;
use crate::data::model::SortColumn;
use crate::data::settings;
use crate::ui::settings::SettingsTab;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	button::{Button, ButtonVariants},
	setting::{SettingField, SettingGroup, SettingItem, SettingPage},
	ActiveTheme, Disableable, Icon, IconName, Sizable,
};

pub fn processes_page(
	view: &Entity<SettingsTab>,
	default_config: &Config,
) -> SettingPage {
	let view = view.clone();
	let default_vram = SharedString::from(
		default_config.processes.behaviour.vram_polling.to_string(),
	);
	let default_filter = SharedString::from(
		default_config
			.processes
			.behaviour
			.pid_filter_mode
			.to_string(),
	);
	let default_resource = SharedString::from(
		default_config
			.processes
			.behaviour
			.resource_view_mode
			.to_string(),
	);
	let default_clear_search =
		default_config.processes.behaviour.clear_search_on_pin;
	let default_sort_col = SharedString::from(
		default_config.processes.default_sort.column.to_string(),
	);
	let default_sort_desc = default_config.processes.default_sort.descending;

	let sort_options: Vec<(SharedString, SharedString)> = SortColumn::all()
		.into_iter()
		.map(|sc| {
			(
				SharedString::from(sc.to_string()),
				SharedString::from(sc.to_string()),
			)
		})
		.collect();

	let column_labels: Vec<(&str, SortColumn)> = vec![
		("Name", SortColumn::Name),
		("Process ID", SortColumn::Pid),
		("User", SortColumn::User),
		("State", SortColumn::State),
		("CPU Usage", SortColumn::Cpu),
		("Memory Usage", SortColumn::Memory),
		("VRAM Usage", SortColumn::Vram),
		("Disk Read", SortColumn::DiskRead),
		("Disk Write", SortColumn::DiskWrite),
	];

	SettingPage::new("Processes")
		.default_open(true)
		.icon(Icon::new(IconName::Cpu))
		.groups(vec![
			SettingGroup::new().title("Behaviour").items(vec![
				SettingItem::new(
					"VRAM Polling",
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
									view.read(cx)
										.config
										.processes
										.behaviour
										.vram_polling
										.to_string(),
								)
							}
						},
						{
							let view = view.clone();
							move |val: SharedString, cx: &mut App| {
								view.update(cx, |this, cx| {
									settings::set_vram_polling(
										&mut this.config,
										&val,
									);
									this.save();
									cx.notify();
								});
							}
						},
					)
					.default_value(default_vram),
				)
				.description(
					"Auto detects the GPU at startup and tracks VRAM if \
					 available. On forces polling. Off disables it \
					 completely.",
				)
				.keywords(["gpu", "memory", "video"]),
				SettingItem::new(
					"Process Tree Filter",
					SettingField::dropdown(
						vec![
							("Direct only".into(), "Direct only".into()),
							(
								"All descendants".into(),
								"All descendants".into(),
							),
						],
						{
							let view = view.clone();
							move |cx: &App| {
								SharedString::from(
									view.read(cx)
										.config
										.processes
										.behaviour
										.pid_filter_mode
										.to_string(),
								)
							}
						},
						{
							let view = view.clone();
							move |val: SharedString, cx: &mut App| {
								view.update(cx, |this, cx| {
									settings::set_pid_filter_mode(
										&mut this.config,
										&val,
									);
									this.save();
									cx.notify();
								});
							}
						},
					)
					.default_value(default_filter),
				)
				.description(
					"When filtering by a process, show only its direct \
					 children or all descendants.",
				)
				.keywords(["children", "pid", "tree"]),
				SettingItem::new(
					"Clear Search on Pin",
					SettingField::switch(
						{
							let view = view.clone();
							move |cx: &App| {
								view.read(cx)
									.config
									.processes
									.behaviour
									.clear_search_on_pin
							}
						},
						{
							let view = view.clone();
							move |val: bool, cx: &mut App| {
								view.update(cx, |this, cx| {
									settings::set_clear_search_on_pin(
										&mut this.config,
										val,
									);
									this.save();
									cx.notify();
								});
							}
						},
					)
					.default_value(default_clear_search),
				)
				.description(
					"When double-clicking a process to filter by it, also \
					 clear the search bar.",
				)
				.keywords(["double-click", "pin", "search"]),
				SettingItem::new(
					"Resource View",
					SettingField::dropdown(
						vec![
							("Self".into(), "Self".into()),
							("Cumulative".into(), "Cumulative".into()),
						],
						{
							let view = view.clone();
							move |cx: &App| {
								SharedString::from(
									view.read(cx)
										.config
										.processes
										.behaviour
										.resource_view_mode
										.to_string(),
								)
							}
						},
						{
							let view = view.clone();
							move |val: SharedString, cx: &mut App| {
								view.update(cx, |this, cx| {
									settings::set_resource_view_mode(
										&mut this.config,
										&val,
									);
									this.save();
									cx.notify();
								});
							}
						},
					)
					.default_value(default_resource),
				)
				.description(
					"Self shows each process's own usage. Cumulative shows \
					 the process totalled with all its descendants.",
				)
				.keywords(["usage", "total", "cumulative"]),
			]),
			SettingGroup::new().title("Columns").items({
				column_labels
					.iter()
					.enumerate()
					.map(|(idx, (title, col))| {
						let view = view.clone();
						let title = *title;
						let col = *col;
						let has_up = idx > 0;
						let has_down = idx < column_labels.len() - 1;
						SettingItem::render(move |_options, _window, cx| {
							let visible = view
								.read(cx)
								.config
								.processes
								.columns
								.iter()
								.find(|e| e.column == col)
								.map(|e| e.visible)
								.unwrap_or(true);

							div()
								.flex()
								.flex_row()
								.items_center()
								.gap(px(8.0))
								.child(
									div()
										.text_sm()
										.text_color(cx.theme().foreground)
										.child(title),
								)
								.child(div().flex_grow(1.0))
								.child(
									Button::new(format!("toggle-{idx}"))
										.ghost()
										.label(if visible {
											"✓"
										} else {
											"✗"
										})
										.xsmall()
										.on_click({
											let view = view.clone();
											move |_, _, cx| {
												view.update(
													cx,
													|this, cx| {
														settings::toggle_column_visibility(
															&mut this
																.config,
															col,
														);
														this.save();
														cx.notify();
													},
												);
											}
										}),
								)
								.child(
									Button::new(format!("move-up-{idx}"))
										.ghost()
										.icon(
											Icon::new(IconName::ChevronUp)
												.size(px(12.0))
												.text_color(
													cx.theme()
														.muted_foreground,
												),
										)
										.xsmall()
										.disabled(!has_up)
										.on_click({
											let view = view.clone();
											move |_, _, cx| {
												view.update(
													cx,
													|this, cx| {
														settings::swap_columns(
															&mut this
																.config,
															idx,
															idx
																- 1,
														);
														this.save();
														cx.notify();
													},
												);
											}
										}),
								)
								.child(
									Button::new(format!("move-down-{idx}"))
										.ghost()
										.icon(
											Icon::new(IconName::ChevronDown)
												.size(px(12.0))
												.text_color(
													cx.theme()
														.muted_foreground,
												),
										)
										.xsmall()
										.disabled(!has_down)
										.on_click({
											let view = view.clone();
											move |_, _, cx| {
												view.update(
													cx,
													|this, cx| {
														settings::swap_columns(
															&mut this
																.config,
															idx,
															idx
																+ 1,
														);
														this.save();
														cx.notify();
													},
												);
											}
										}),
								)
								.into_any()
						})
					})
					.collect::<Vec<SettingItem>>()
			}),
			SettingGroup::new().title("Default Sort").items(vec![
				SettingItem::new(
					"Sort Column",
					SettingField::dropdown(
						sort_options,
						{
							let view = view.clone();
							move |cx: &App| {
								SharedString::from(
									view.read(cx)
										.config
										.processes
										.default_sort
										.column
										.to_string(),
								)
							}
						},
						{
							let view = view.clone();
							move |val: SharedString, cx: &mut App| {
								view.update(cx, |this, cx| {
									settings::set_sort_column(
										&mut this.config,
										&val,
									);
									this.save();
									cx.notify();
								});
							}
						},
					)
					.default_value(default_sort_col),
				)
				.description("Default column to sort the process table by.")
				.keywords(["order", "sorting"]),
				SettingItem::new(
					"Sort Descending",
					SettingField::switch(
						{
							let view = view.clone();
							move |cx: &App| {
								view.read(cx)
									.config
									.processes
									.default_sort
									.descending
							}
						},
						{
							let view = view.clone();
							move |val: bool, cx: &mut App| {
								view.update(cx, |this, cx| {
									settings::set_sort_descending(
										&mut this.config,
										val,
									);
									this.save();
									cx.notify();
								});
							}
						},
					)
					.default_value(default_sort_desc),
				)
				.description(
					"Sort in descending order by default (largest values \
					 first).",
				)
				.keywords(["reverse", "direction", "ascending"]),
			]),
		])
}
