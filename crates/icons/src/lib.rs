use std::collections::HashMap;

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

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::{load_cache, resolve_icon_path};
#[cfg(target_os = "macos")]
pub use macos::{load_cache, resolve_icon_path};
#[cfg(target_os = "windows")]
pub use windows::{load_cache, resolve_icon_path};
