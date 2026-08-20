use crate::mutations as settings;
use crate::tab::SettingsTab;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	button::{Button, ButtonVariants},
	menu::{DropdownMenu, PopupMenuItem},
	setting::{SettingField, SettingGroup, SettingItem, SettingPage},
	ActiveTheme, Disableable, Sizable,
};
use gpuitop_components::assets::lucide::LucideIcon;
use gpuitop_core::config::Config;
use gpuitop_core::model::SortColumn;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub fn processes_page(
	view: &Entity<SettingsTab>,
	default_config: &Config,
	gpu_data: &Arc<AtomicBool>,
	redetect: &Arc<AtomicBool>,
) -> SettingPage {
	let view = view.clone();
	let gpu_data = gpu_data.clone();
	let redetect = redetect.clone();
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
	let default_view = SharedString::from(
		default_config
			.processes
			.behaviour
			.default_view_mode
			.to_string(),
	);
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
		.icon(LucideIcon::List.icon())
		.groups(vec![
			SettingGroup::new().title("Behaviour").items(vec![
				SettingItem::new(
					"GPU Data",
					SettingField::render({
						let view = view.clone();
						move |_options, _window, cx| {
							let gpu_data = gpu_data.clone();
							let redetect = redetect.clone();
							let enabled = gpu_data.load(Ordering::SeqCst);
							let old_value = view
								.read(cx)
								.config
								.get()
								.processes
								.behaviour
								.gpu_data
								.to_string();
							div()
								.flex()
								.flex_row()
								.items_center()
								.gap(px(8.0))
								.child(
									Button::new("gpu-redetect")
										.label("Redetect GPU Backends")
										.xsmall()
										.disabled(!enabled)
										.on_click(move |_, _, _| {
											redetect.store(
												true,
												Ordering::SeqCst,
											);
										}),
								)
								.child(
									Button::new("gpu-data-dropdown")
										.label(old_value.clone())
										.dropdown_caret(true)
										.outline()
										.small()
										.dropdown_menu_with_anchor(
											gpui::Anchor::TopRight,
											{
												let view = view.clone();
												let gpu_data =
													gpu_data.clone();
												let old_value =
													old_value.clone();
												move |menu, _, _| {
													menu.item(
														PopupMenuItem::new(
															"On",
														)
														.checked(
															old_value == "On",
														)
														.on_click({
															let view =
																view.clone();
															let gpu_data =
																gpu_data
																	.clone();
															move |_, _, cx| {
																let gpu_data =
																	gpu_data
																		.clone(
																		);
																view.update(
																cx,
																move |this,
																		cx| {
																	this.config.mutate(|c| {
																		settings::set_gpu_data(c, &gpu_data, "On");
																	});
																	cx.notify();
																},
															);
															}
														}),
													)
													.item(
														PopupMenuItem::new(
															"Off",
														)
														.checked(
															old_value
																== "Off",
														)
														.on_click({
															let view =
																view.clone();
															let gpu_data =
																gpu_data
																	.clone();
															move |_, _, cx| {
																let gpu_data =
																	gpu_data
																		.clone(
																		);
																view.update(
																cx,
																move |this,
																		cx| {
																	this.config.mutate(|c| {
																		settings::set_gpu_data(c, &gpu_data, "Off");
																	});
																	cx.notify();
																},
															);
															}
														}),
													)
												}
											},
										),
								)
								.into_any()
						}
					}),
				)
				.description(
					"On collects GPU data (per-PID VRAM and per-device \
					 telemetry) every tick, re-probing for GPUs until one \
					 is found. Off disables it completely.",
				)
				.keywords(["gpu", "vram", "memory", "video"]),
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
										.get()
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
									this.config.mutate(|c| {
										settings::set_pid_filter_mode(
											c, &val,
										);
									});
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
									.get()
									.processes
									.behaviour
									.clear_search_on_pin
							}
						},
						{
							let view = view.clone();
							move |val: bool, cx: &mut App| {
								view.update(cx, |this, cx| {
									this.config.mutate(|c| {
										settings::set_clear_search_on_pin(
											c, val,
										);
									});
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
										.get()
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
									this.config.mutate(|c| {
										settings::set_resource_view_mode(
											c, &val,
										);
									});
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
				SettingItem::new(
					"Default View",
					SettingField::dropdown(
						vec![
							("List".into(), "List".into()),
							("Tree".into(), "Tree".into()),
						],
						{
							let view = view.clone();
							move |cx: &App| {
								SharedString::from(
									view.read(cx)
										.config
										.get()
										.processes
										.behaviour
										.default_view_mode
										.to_string(),
								)
							}
						},
						{
							let view = view.clone();
							move |val: SharedString, cx: &mut App| {
								view.update(cx, |this, cx| {
									this.config.mutate(|c| {
										settings::set_default_view_mode(
											c, &val,
										);
									});
									cx.notify();
								});
							}
						},
					)
					.default_value(default_view),
				)
				.description(
					"Choose whether the processes tab starts in list or \
					 tree view.",
				)
				.keywords(["tree", "list", "view", "default"]),
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
								.get()
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
														this.config.mutate(
															|c| {
																settings::toggle_column_visibility(c, col);
															},
														);
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
											LucideIcon::ChevronUp
												.icon()
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
														this.config.mutate(
															|c| {
																settings::swap_columns(c, idx, idx - 1);
															},
														);
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
											LucideIcon::ChevronDown
												.icon()
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
														this.config.mutate(
															|c| {
																settings::swap_columns(c, idx, idx + 1);
															},
														);
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
										.get()
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
									this.config.mutate(|c| {
										settings::set_sort_column(c, &val);
									});
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
									.get()
									.processes
									.default_sort
									.descending
							}
						},
						{
							let view = view.clone();
							move |val: bool, cx: &mut App| {
								view.update(cx, |this, cx| {
									this.config.mutate(|c| {
										settings::set_sort_descending(c, val);
									});
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
