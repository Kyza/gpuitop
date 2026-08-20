use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DesktopEntry {
	pub name: String,
	pub icon_name: String,
	pub exec: String,
}

#[derive(Debug, Clone, Default)]
pub struct DesktopEntryCache {
	entries: Vec<DesktopEntry>,
	by_exec: HashMap<String, usize>,
	by_token: HashMap<String, usize>,
	by_name: Vec<(String, usize)>,
}

impl DesktopEntryCache {
	#[hotpath::measure]
	pub fn lookup(
		&self,
		cmdline: &str,
		exe_basename: &str,
	) -> Option<&DesktopEntry> {
		// Exact exec-basename hit.
		if let Some(&idx) =
			self.by_exec.get(exe_basename.to_lowercase().as_str())
		{
			return Some(&self.entries[idx]);
		}

		// Path-component scan of the cmdline. Only tokens that are path
		// components (inside a `/`-delimited word) or dotted app-ids
		// (org.signal.Signal) count — a bare flag argument like `vesktop`
		// in `--search vesktop` must never match the vesktop entry. This is
		// the net that catches Electron helpers, whose cmdline carries the
		// app path (/usr/lib/vesktop/app.asar). Longest key wins (most
		// specific); user-dir entries win ties (inserted later, so higher
		// index).
		let mut best: Option<(usize, usize)> = None;
		for word in cmdline.split_whitespace() {
			if word.contains('/') {
				for seg in word.split('/') {
					if seg.is_empty() {
						continue;
					}
					if let Some(&idx) =
						self.by_token.get(seg.to_lowercase().as_str())
					{
						let len = seg.len();
						if best.is_none_or(|(bl, bi)| {
							len > bl || (len == bl && idx > bi)
						}) {
							best = Some((len, idx));
						}
					}
				}
			} else if word.contains('.') {
				if let Some(&idx) =
					self.by_token.get(word.to_lowercase().as_str())
				{
					let len = word.len();
					if best.is_none_or(|(bl, bi)| {
						len > bl || (len == bl && idx > bi)
					}) {
						best = Some((len, idx));
					}
				}
			}
		}
		if let Some((_, idx)) = best {
			return Some(&self.entries[idx]);
		}

		// exe_basename starts with a normalized desktop Name (kate-server).
		let base = exe_basename.to_lowercase();
		let mut best: Option<(usize, usize)> = None;
		for (name, idx) in &self.by_name {
			if base.starts_with(name.as_str()) {
				let len = name.len();
				if best.is_none_or(|(bl, bi)| {
					len > bl || (len == bl && *idx > bi)
				}) {
					best = Some((len, *idx));
				}
			}
		}
		best.map(|(_, idx)| &self.entries[idx])
	}
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::{load_cache, resolve_icon_path, warm_icon_paths};
#[cfg(target_os = "macos")]
pub use macos::{load_cache, resolve_icon_path, warm_icon_paths};
#[cfg(target_os = "windows")]
pub use windows::{load_cache, resolve_icon_path, warm_icon_paths};
