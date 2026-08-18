use crate::delegate::ProcessTableDelegate;
use bytesize::ByteSize;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	menu::PopupMenu,
	table::{Column, ColumnFixed, ColumnSort, TableDelegate, TableState},
	ActiveTheme,
};
use gpuitop_components::assets::lucide::LucideIcon;
use gpuitop_components::theme::{state_info, tag_icons, theme_dark_or_light};
use gpuitop_core::state::ViewState;
use gpuitop_icons::resolve_icon_path;

impl TableDelegate for ProcessTableDelegate {
	fn columns_count(&self, _: &App) -> usize {
		10
	}

	fn rows_count(&self, _: &App) -> usize {
		self.filtered_sorted_rows().len()
	}

	fn column(&self, col_ix: usize, _: &App) -> Column {
		let hidden = self.is_col_hidden(col_ix);
		let (key, name, width, sort) = match col_ix {
			0 => ("tags", " ", 64.0, None),
			1 => ("name", "Name", 260.0, Some(ColumnSort::Ascending)),
			2 => ("pid", "PID", 72.0, Some(ColumnSort::Ascending)),
			3 => ("user", "User", 80.0, Some(ColumnSort::Ascending)),
			4 => ("state", "State", 72.0, None),
			5 => ("cpu", "CPU%", 90.0, Some(ColumnSort::Descending)),
			6 => ("mem", "Mem", 88.0, Some(ColumnSort::Descending)),
			7 => ("vram", "VRAM", 88.0, Some(ColumnSort::Descending)),
			8 => ("dread", "Disk R", 96.0, Some(ColumnSort::Descending)),
			9 => ("dwrite", "Disk W", 96.0, Some(ColumnSort::Descending)),
			_ => ("", "", 100.0, None),
		};
		Column {
			key: key.into(),
			name: name.into(),
			width: if hidden { px(0.0) } else { px(width) },
			sort: if hidden { None } else { sort },
			resizable: !hidden && col_ix != 0,
			movable: !hidden && col_ix != 0,
			fixed: if col_ix == 0 {
				Some(ColumnFixed::Left)
			} else {
				None
			},
			min_width: px(24.0),
			max_width: px(800.0),
			..Default::default()
		}
	}

	#[hotpath::measure]
	fn render_th(
		&mut self,
		col_ix: usize,
		_window: &mut Window,
		cx: &mut Context<TableState<Self>>,
	) -> impl IntoElement {
		if self.is_col_hidden(col_ix) {
			return div().into_any_element();
		}
		let icon = match col_ix {
			1 => Some(LucideIcon::File),
			2 => Some(LucideIcon::Hash),
			3 => Some(LucideIcon::User),
			4 => Some(LucideIcon::Activity),
			5 => Some(LucideIcon::Cpu),
			6 => Some(LucideIcon::MemoryStick),
			7 => Some(LucideIcon::Gpu),
			8 => Some(LucideIcon::HardDriveDownload),
			9 => Some(LucideIcon::HardDriveUpload),
			_ => None,
		};
		let name = self.column(col_ix, cx).name.clone();
		div()
			.flex()
			.flex_row()
			.items_center()
			.gap(px(4.0))
			.when(icon.is_some(), |el| {
				el.child(
					icon.unwrap()
						.icon()
						.size(px(12.0))
						.text_color(cx.theme().muted_foreground),
				)
			})
			.child(name)
			.into_any_element()
	}

	#[hotpath::measure]
	fn render_td(
		&mut self,
		row_ix: usize,
		col_ix: usize,
		_: &mut Window,
		cx: &mut Context<TableState<Self>>,
	) -> impl IntoElement {
		if self.is_col_hidden(col_ix) {
			return div().into_any();
		}
		let rows = self.filtered_sorted_rows();
		let Some(proc) = rows.get(row_ix) else {
			return div().into_any();
		};

		let cum = self
			.cum_cache
			.borrow()
			.as_ref()
			.and_then(|m| m.get(&proc.pid))
			.cloned();

		match col_ix {
			0 => {
				let tags = tag_icons(proc, self.init_system, cx);
				let is_pinned = self.pinned_pid() == Some(proc.pid);
				div()
					.flex()
					.flex_row()
					.gap(px(2.0))
					.items_center()
					.when(is_pinned, |el| {
						el.child(
							LucideIcon::Pin
								.icon()
								.w(px(12.0))
								.h(px(12.0))
								.text_color(cx.theme().primary),
						)
					})
					.children(tags.into_iter().enumerate().map(
						|(i, (icon, color, tip_label))| {
							let el = div().child(
								icon.icon()
									.w(px(12.0))
									.h(px(12.0))
									.text_color(color),
							);
							let el = match tip_label {
								Some(tip) => el
									.id(format!("tag-{}-{}", proc.pid, i))
									.tooltip(move |window, cx| {
										gpui_component::tooltip::Tooltip::new(
											tip,
										)
										.build(window, cx)
									}),
								None => {
									el.id(format!("tag-{}-{}", proc.pid, i))
								}
							};
							el.into_any_element()
						},
					))
					.into_any()
			}
			1 => {
				let has_children = proc.has_children;
				let descendant_count = self.count_descendants_of(proc.pid);
				let show_badge = has_children && descendant_count > 0;
				let display_name = proc
					.electron_app_name
					.as_deref()
					.unwrap_or(&proc.name)
					.to_string();
				let icon_path =
					proc.icon_name.as_deref().and_then(resolve_icon_path);

				let name_el = div()
					.flex()
					.flex_row()
					.items_center()
					.gap(px(4.0))
					.text_sm()
					.text_color(cx.theme().foreground)
					.when_some(icon_path, |el, path| {
						el.child(
							div()
								.w(px(16.0))
								.h(px(16.0))
								.flex()
								.items_center()
								.justify_center()
								.child(
									img(path).object_fit(ObjectFit::Contain),
								),
						)
					})
					.child(display_name.clone());

				if show_badge {
					name_el
						.child(
							div()
								.text_size(px(10.0))
								.text_color(
									cx.theme().muted_foreground.opacity(0.6),
								)
								.ml(px(4.0))
								.child(format!("(+{descendant_count})")),
						)
						.into_any()
				} else {
					name_el.into_any()
				}
			}
			2 => div()
				.text_sm()
				.text_color(cx.theme().foreground)
				.child(proc.pid.to_string())
				.into_any(),
			3 => div()
				.text_sm()
				.text_color(cx.theme().muted_foreground)
				.child(proc.user.clone())
				.into_any(),
			4 => {
				let (state_label, c) =
					state_info(proc.state, theme_dark_or_light(cx));
				div().text_sm().text_color(c).child(state_label).into_any()
			}
			5 => {
				let cpu_val =
					cum.as_ref().map_or(proc.cpu_percent, |c| c.cpu);
				let c = if cpu_val > 50.0 {
					cx.theme().danger
				} else if cpu_val > 20.0 {
					cx.theme().warning
				} else {
					cx.theme().foreground
				};
				div()
					.text_sm()
					.text_color(c)
					.text_align(TextAlign::Right)
					.child(format!("{:.1}", cpu_val))
					.into_any()
			}
			6 => {
				let mem_val =
					cum.as_ref().map_or(proc.mem_rss, |c| c.mem_rss);
				let mem_pct = if let Some(ref c) = cum {
					let total_mem = self.snapshot_cell.borrow().memory.total;
					if total_mem > 0 {
						c.mem_rss as f32 / total_mem as f32 * 100.0
					} else {
						0.0
					}
				} else {
					proc.mem_percent
				};
				let c = if mem_pct > 30.0 {
					cx.theme().danger
				} else if mem_pct > 10.0 {
					cx.theme().warning
				} else {
					cx.theme().foreground
				};
				div()
					.text_sm()
					.text_color(c)
					.text_align(TextAlign::Right)
					.child(ByteSize::b(mem_val).to_string())
					.into_any()
			}
			7 => {
				let vram = cum.as_ref().map_or(proc.vram, |c| c.vram);
				let breakdown = format!(
					"NVIDIA: {} · AMD: {}",
					ByteSize::b(vram.nvidia),
					ByteSize::b(vram.amd)
				);
				div()
					.text_sm()
					.text_color(cx.theme().muted_foreground)
					.text_align(TextAlign::Right)
					.child(if vram.is_empty() {
						"—".into()
					} else {
						ByteSize::b(vram.total()).to_string()
					})
					.id(format!("vram-{}", proc.pid))
					.tooltip(move |window, cx| {
						gpui_component::tooltip::Tooltip::new(
							breakdown.clone(),
						)
						.build(window, cx)
					})
					.into_any()
			}
			8 => {
				let dread_val = cum
					.as_ref()
					.map_or(proc.disk_read_bytes_per_sec, |c| c.disk_read);
				div()
					.text_sm()
					.text_color(cx.theme().muted_foreground)
					.text_align(TextAlign::Right)
					.child(if dread_val > 0.0 {
						format!("{}/s", ByteSize::b(dread_val as u64))
					} else {
						"0".into()
					})
					.into_any()
			}
			9 => {
				let dwrite_val = cum
					.as_ref()
					.map_or(proc.disk_write_bytes_per_sec, |c| c.disk_write);
				div()
					.text_sm()
					.text_color(cx.theme().muted_foreground)
					.text_align(TextAlign::Right)
					.child(if dwrite_val > 0.0 {
						format!("{}/s", ByteSize::b(dwrite_val as u64))
					} else {
						"0".into()
					})
					.into_any()
			}
			_ => div().into_any(),
		}
	}

	fn perform_sort(
		&mut self,
		col_ix: usize,
		sort: ColumnSort,
		_: &mut Window,
		cx: &mut Context<TableState<Self>>,
	) {
		if self.is_col_hidden(col_ix) {
			return;
		}
		let dir = match sort {
			ColumnSort::Ascending => {
				gpuitop_core::model::SortDirection::Ascending
			}
			ColumnSort::Descending => {
				gpuitop_core::model::SortDirection::Descending
			}
			_ => gpuitop_core::model::SortDirection::Ascending,
		};
		ViewState::mutate(&self.view_state, |s| {
			s.sort_col = col_ix;
			s.sort_dir = dir;
		});
		cx.notify();
	}

	fn context_menu(
		&mut self,
		row_ix: usize,
		mut menu: PopupMenu,
		_window: &mut Window,
		_cx: &mut Context<TableState<Self>>,
	) -> PopupMenu {
		let rows = self.filtered_sorted_rows();
		if let Some(proc) = rows.get(row_ix) {
			for item in crate::context_menu::build_process_menu(proc) {
				menu = menu.item(item);
			}
		}
		menu
	}

	fn render_tr(
		&mut self,
		row_ix: usize,
		_window: &mut Window,
		_cx: &mut Context<TableState<Self>>,
	) -> Stateful<Div> {
		div().id(("row", row_ix))
	}

	fn render_empty(
		&mut self,
		_: &mut Window,
		cx: &mut Context<TableState<Self>>,
	) -> impl IntoElement {
		div()
			.size_full()
			.flex()
			.items_center()
			.justify_center()
			.py(px(32.0))
			.text_sm()
			.text_color(cx.theme().muted_foreground)
			.child("No processes match the current filters")
			.into_any_element()
	}
}
