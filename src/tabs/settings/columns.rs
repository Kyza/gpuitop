use crate::config::{ColumnVisibility, Config};
use crate::tabs::settings::SettingsTab;
use gpui::*;
use gpui_component::{
	setting::{SettingField, SettingGroup, SettingItem, SettingPage},
	Icon, IconName,
};

pub fn columns_page(
	view: &Entity<SettingsTab>,
	default_config: &Config,
) -> SettingPage {
	let view = view.clone();
	let default_cols = default_config.columns.clone();

	let column_items: Vec<(&str, &str, fn(&ColumnVisibility) -> bool, fn(&mut ColumnVisibility, bool), &[&str])> = vec![
		("Process ID", "The numeric process identifier.", |c| c.pid, |c, v| c.pid = v, &["pid", "id"]),
		("User", "The username that owns the process.", |c| c.user, |c, v| c.user = v, &["username", "owner"]),
		("State", "Process state (Running, Sleeping, Zombie, etc.).", |c| c.state, |c, v| c.state = v, &["status", "zombie"]),
		("CPU usage", "Percentage of CPU used by the process.", |c| c.cpu, |c, v| c.cpu = v, &["processor", "cpu_usage"]),
		("Memory usage", "Resident memory used by the process.", |c| c.memory, |c, v| c.memory = v, &["ram", "rss", "mem"]),
		("VRAM usage", "Dedicated GPU memory used by the process.", |c| c.vram, |c, v| c.vram = v, &["gpu", "video", "graphics"]),
		("Disk read", "Bytes read from disk.", |c| c.disk_read, |c, v| c.disk_read = v, &["io", "read_bytes"]),
		("Disk write", "Bytes written to disk.", |c| c.disk_write, |c, v| c.disk_write = v, &["io", "write_bytes"]),
		("Full command", "The complete command line of the process.", |c| c.command, |c, v| c.command = v, &["cmd", "cmdline", "args"]),
	];

	let items: Vec<SettingItem> = column_items
		.into_iter()
		.map(|(title, desc, getter, setter, keywords)| {
			let view = view.clone();
			let default_enabled = getter(&default_cols);
			let keywords: Vec<&str> = keywords.iter().copied().collect();
			SettingItem::new(
				title,
				SettingField::switch(
					{
						let view = view.clone();
						move |cx: &App| {
							getter(&view.read(cx).config.columns)
						}
					},
					{
						let view = view.clone();
						move |val: bool, cx: &mut App| {
							view.update(cx, |this, cx| {
								setter(&mut this.config.columns, val);
								this.save();
								cx.notify();
							});
						}
					},
				)
				.default_value(default_enabled),
			)
			.description(desc)
			.keywords(keywords)
		})
		.collect();

	SettingPage::new("Columns")
		.default_open(false)
		.icon(Icon::new(IconName::LayoutDashboard))
		.group(
			SettingGroup::new()
				.title("Process Table Columns")
				.items(items),
		)
}
