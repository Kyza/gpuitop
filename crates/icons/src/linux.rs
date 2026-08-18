use std::fs;
use std::path::{Path, PathBuf};

use super::{DesktopEntry, DesktopEntryCache};

fn desktop_dirs() -> Vec<PathBuf> {
	let mut dirs = vec![PathBuf::from("/usr/share/applications")];
	if let Ok(home) = std::env::var("HOME") {
		dirs.push(PathBuf::from(home).join(".local/share/applications"));
	}
	dirs
}

fn parse_desktop_file(path: &Path) -> Option<DesktopEntry> {
	let data = fs::read_to_string(path).ok()?;
	let mut in_entry = false;
	let mut name = String::new();
	let mut icon_name = String::new();
	let mut exec = String::new();

	for line in data.lines() {
		let trimmed = line.trim();
		if trimmed == "[Desktop Entry]" {
			in_entry = true;
			continue;
		}
		if trimmed.starts_with('[') {
			in_entry = false;
			continue;
		}
		if !in_entry {
			continue;
		}
		if trimmed.starts_with("Name=") && name.is_empty() {
			name = trimmed[5..].to_string();
		} else if trimmed.starts_with("Icon=") && icon_name.is_empty() {
			icon_name = trimmed[5..].to_string();
		} else if trimmed.starts_with("Exec=") && exec.is_empty() {
			exec = trimmed[5..].to_string();
		}
	}

	if exec.is_empty() {
		return None;
	}
	if icon_name.is_empty() {
		icon_name = name.to_lowercase();
	}
	if name.is_empty() {
		name = path
			.file_stem()
			.and_then(|s| s.to_str())
			.unwrap_or("Unknown")
			.to_string();
	}

	Some(DesktopEntry {
		name,
		icon_name,
		exec,
	})
}

fn extract_exec_basename(exec: &str) -> Option<String> {
	let exe = exec.split_whitespace().next()?.split(" %").next()?;
	let path = Path::new(exe);
	path.file_name()
		.and_then(|n| n.to_str())
		.map(|s| s.to_string())
}

fn scan_dir(cache: &mut DesktopEntryCache, dir: &Path) {
	let Ok(entries) = fs::read_dir(dir) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		if path.extension().map_or(true, |e| e != "desktop") {
			continue;
		}
		if let Some(de) = parse_desktop_file(&path) {
			if let Some(exec_bin) = extract_exec_basename(&de.exec) {
				let key = exec_bin.to_lowercase();
				cache.by_exec.insert(key, de);
			}
		}
	}
}

pub fn load_cache() -> DesktopEntryCache {
	let mut cache = DesktopEntryCache::default();
	for dir in desktop_dirs() {
		scan_dir(&mut cache, &dir);
	}
	cache
}

pub fn resolve_icon_path(icon_name: &str) -> Option<PathBuf> {
	if icon_name.is_empty() {
		return None;
	}
	if icon_name.starts_with('/') {
		let p = PathBuf::from(icon_name);
		if p.exists() {
			return Some(p);
		}
		return None;
	}

	let sizes = [256, 128, 96, 64, 48, 32, 24, 22, 16];
	let exts = ["png", "svg", "xpm"];

	for size in &sizes {
		for ext in &exts {
			let p = PathBuf::from(format!(
				"/usr/share/icons/hicolor/{}x{}/apps/{}.{}",
				size, size, icon_name, ext
			));
			if p.exists() {
				return Some(p);
			}
		}
	}

	for ext in &exts {
		let p = PathBuf::from(format!(
			"/usr/share/pixmaps/{}.{}",
			icon_name, ext
		));
		if p.exists() {
			return Some(p);
		}
	}

	None
}

#[cfg(test)]
mod tests {
	use super::extract_exec_basename;
	use crate::{resolve_icon_path, DesktopEntry, DesktopEntryCache};

	#[test]
	fn test_extract_exec_basename_full_path() {
		assert_eq!(
			extract_exec_basename("/usr/lib/firefox/firefox %u"),
			Some("firefox".into())
		);
	}

	#[test]
	fn test_extract_exec_basename_bare_command() {
		assert_eq!(
			extract_exec_basename("balena-etcher %U"),
			Some("balena-etcher".into())
		);
	}

	#[test]
	fn test_extract_exec_basename_no_args() {
		assert_eq!(
			extract_exec_basename("/opt/discord/discord"),
			Some("discord".into())
		);
	}

	#[test]
	fn test_extract_exec_basename_single_token() {
		assert_eq!(extract_exec_basename("code"), Some("code".into()));
	}

	#[test]
	fn test_desktop_entry_cache_lookup_exact() {
		let mut cache = DesktopEntryCache::default();
		cache.by_exec.insert(
			"firefox".into(),
			DesktopEntry {
				name: "Firefox".into(),
				icon_name: "firefox".into(),
				exec: "/usr/lib/firefox/firefox %u".into(),
			},
		);
		let result = cache.lookup("", "firefox");
		assert!(result.is_some());
		assert_eq!(result.unwrap().icon_name, "firefox");
	}

	#[test]
	fn test_desktop_entry_cache_lookup_cmdline_fallback() {
		let mut cache = DesktopEntryCache::default();
		cache.by_exec.insert(
			"discord".into(),
			DesktopEntry {
				name: "Discord".into(),
				icon_name: "discord".into(),
				exec: "/opt/Discord/Discord".into(),
			},
		);
		let result = cache.lookup("/opt/Discord/Discord --type=renderer", "");
		assert!(result.is_some());
	}

	#[test]
	fn test_desktop_entry_cache_lookup_none() {
		let cache = DesktopEntryCache::default();
		let result = cache.lookup("", "unknown-binary");
		assert!(result.is_none());
	}

	#[test]
	fn test_resolve_icon_path_empty() {
		assert_eq!(resolve_icon_path(""), None);
	}

	#[test]
	fn test_resolve_icon_path_absolute() {
		let result = resolve_icon_path("/nonexistent/icon.png");
		assert_eq!(result, None);
	}

	#[test]
	fn test_resolve_icon_path_unknown() {
		let result = resolve_icon_path("zzz_nonexistent_icon_name_12345");
		assert_eq!(result, None);
	}
}
