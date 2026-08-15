use std::collections::HashMap;
use std::path::PathBuf;

use oswap::{define_interface, define_platforms};

#[derive(Debug, Clone)]
pub struct DesktopEntry {
	pub name: String,
	pub icon_name: String,
	pub exec: String,
}

#[derive(Debug, Clone, Default)]
pub struct DesktopEntryCache {
	by_exec: HashMap<String, DesktopEntry>,
}

impl DesktopEntryCache {
	pub fn lookup(
		&self,
		cmdline: &str,
		exe_basename: &str,
	) -> Option<&DesktopEntry> {
		let key = exe_basename.to_lowercase();
		if let Some(de) = self.by_exec.get(&key) {
			return Some(de);
		}
		let lower = cmdline.to_lowercase();
		for (exec_bin, de) in &self.by_exec {
			if lower.contains(exec_bin.as_str()) {
				return Some(de);
			}
		}
		None
	}
}

define_interface! { Platform, IconsInterface, impl_interface,
	pub fn load_cache() -> DesktopEntryCache;
	pub fn resolve_icon_path(icon_name: &str) -> Option<PathBuf>;
}

define_platforms![
	{ file: "linux", cfg: target_os = "linux" },
	{ file: "macos", cfg: target_os = "macos" },
	{ file: "windows", cfg: target_os = "windows" },
];
