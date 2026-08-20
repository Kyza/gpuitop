use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use super::{DesktopEntry, DesktopEntryCache};

// Resolved icon path cache, keyed by icon_name and invalidated wholesale when
// the system icon theme changes. Resolution is linicon's XDG-aware lookup
// (current theme -> fallbacks -> hicolor); we memoize it so the render thread
// never pays linicon's per-call index parse.
#[derive(Default)]
struct IconPathCache {
	theme: Option<String>,
	last_theme_check: Option<Instant>,
	paths: HashMap<String, Option<PathBuf>>,
}

static CACHE: OnceLock<Mutex<IconPathCache>> = OnceLock::new();

fn cache() -> &'static Mutex<IconPathCache> {
	CACHE.get_or_init(|| Mutex::new(IconPathCache::default()))
}

/// Resolve an icon_name to a path, cache-only. Returns None when not yet
/// resolved — the collector warms the cache asynchronously on its own thread,
/// so the first frame renders without icons and they pop in over a few ticks.
/// Never performs the (ms-scale) linicon lookup on the render thread.
#[hotpath::measure]
pub fn resolve_icon_path(icon_name: &str) -> Option<PathBuf> {
	if icon_name.is_empty() {
		return None;
	}
	if icon_name.starts_with('/') {
		let p = PathBuf::from(icon_name);
		return if p.exists() { Some(p) } else { None };
	}
	cache()
		.lock()
		.unwrap()
		.paths
		.get(icon_name)
		.cloned()
		.flatten()
}

/// Resolve a batch of icon names into the cache. Called from the collector
/// thread after each tick (async warmup). Re-checks the system icon theme at
/// most once per second; a theme change drops the whole cache so stale theme
/// paths never survive a switch.
#[hotpath::measure]
pub fn warm_icon_paths(names: &[String]) {
	let now = Instant::now();
	let extra = extra_search_paths();

	let pending: Vec<String> = {
		let mut c = cache().lock().unwrap();
		if c.last_theme_check
			.map_or(true, |t| now.duration_since(t).as_secs() >= 1)
		{
			c.last_theme_check = Some(now);
			let theme = linicon::get_system_theme();
			if c.theme != theme {
				c.theme = theme;
				c.paths.clear();
			}
		}
		names
			.iter()
			.filter(|n| !c.paths.contains_key(n.as_str()))
			.cloned()
			.collect()
	};

	for name in &pending {
		let path = linicon_lookup(name, &extra);
		cache().lock().unwrap().paths.insert(name.clone(), path);
	}
}

fn extra_search_paths() -> Vec<String> {
	let mut paths = Vec::new();
	if let Ok(home) = std::env::var("HOME") {
		if !home.is_empty() {
			paths.push(format!("{home}/.local/share/icons"));
		}
	}
	if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
		if !xdg.is_empty() {
			paths.push(format!("{xdg}/icons"));
		}
	}
	paths
}

fn linicon_lookup(icon_name: &str, extra: &[String]) -> Option<PathBuf> {
	let refs: Vec<&str> = extra.iter().map(|s| s.as_str()).collect();
	linicon::lookup_icon(icon_name)
		.with_search_paths(&refs)
		.ok()?
		.next()
		.and_then(|r| r.ok())
		.map(|p| p.path)
}

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

/// Resolve the real command a desktop entry launches.
///
/// Handles the wrapper patterns that would otherwise poison the exec key:
/// `env VAR=... cmd` (skip the env and assignments), `flatpak run
/// --command=CMD` (take `--command`). `sh -c`/`bash -c` wrappers are
/// skipped entirely — the quoted command is not worth parsing and `sh` is
/// a dangerously short index key.
fn extract_exec_basename(exec: &str) -> Option<String> {
	let mut tokens = exec.split_whitespace();
	let first = tokens.next()?;
	let first = if first == "env" {
		tokens.find(|t| !t.contains('='))?
	} else {
		first
	};

	let is_flatpak = first == "flatpak" || first.ends_with("/flatpak");
	let exe = if is_flatpak {
		tokens.find_map(|t| t.strip_prefix("--command="))?
	} else {
		first
	};

	if exe == "sh" || exe == "bash" {
		return None;
	}

	let exe = exe.split('%').next()?;
	let path = Path::new(exe);
	path.file_name()
		.and_then(|n| n.to_str())
		.map(|s| s.to_string())
}

/// The flatpak app-id (`org.signal.Signal`) when Exec runs through flatpak.
fn extract_flatpak_app_id(exec: &str) -> Option<String> {
	let mut tokens = exec.split_whitespace();
	let first = tokens.next()?;
	if first != "flatpak" && !first.ends_with("/flatpak") {
		return None;
	}
	tokens
		.find(|t| t.contains('.') && !t.starts_with('-'))
		.map(|s| s.to_string())
}

fn insert_entry(cache: &mut DesktopEntryCache, de: DesktopEntry) {
	let idx = cache.entries.len();
	cache.entries.push(de);
	let de = &cache.entries[idx];

	if let Some(bin) = extract_exec_basename(&de.exec) {
		let key = bin.to_lowercase();
		cache.by_exec.insert(key.clone(), idx);
		cache.by_token.insert(key, idx);
	}
	if let Some(app_id) = extract_flatpak_app_id(&de.exec) {
		let key = app_id.to_lowercase();
		cache.by_token.insert(key, idx);
	}
	let name = de.name.to_lowercase();
	cache.by_token.insert(name.clone(), idx);
	cache.by_name.push((name, idx));
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
			insert_entry(cache, de);
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

#[cfg(test)]
mod tests {
	use super::{extract_exec_basename, insert_entry};
	use crate::{
		resolve_icon_path, warm_icon_paths, DesktopEntry, DesktopEntryCache,
	};

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
	fn test_extract_exec_basename_env_wrapper() {
		assert_eq!(
			extract_exec_basename("env DESKTOPINTEGRATION=1 AyuGram -- %U"),
			Some("AyuGram".into())
		);
	}

	#[test]
	fn test_extract_exec_basename_flatpak() {
		assert_eq!(
			extract_exec_basename(
				"/usr/bin/flatpak run --branch=stable \
				 --command=signal-desktop org.signal.Signal %U"
			),
			Some("signal-desktop".into())
		);
	}

	#[test]
	fn test_extract_exec_basename_sh_wrapper_skipped() {
		assert_eq!(extract_exec_basename("sh -c \"scrcpy\""), None);
	}

	fn entry(name: &str, icon_name: &str, exec: &str) -> DesktopEntry {
		DesktopEntry {
			name: name.into(),
			icon_name: icon_name.into(),
			exec: exec.into(),
		}
	}

	#[test]
	fn test_lookup_exact_exec_basename() {
		let mut cache = DesktopEntryCache::default();
		insert_entry(
			&mut cache,
			entry("Firefox", "firefox", "/usr/lib/firefox/firefox %u"),
		);
		let result = cache.lookup("", "firefox");
		assert!(result.is_some());
		assert_eq!(result.unwrap().icon_name, "firefox");
	}

	#[test]
	fn test_lookup_cmdline_token() {
		let mut cache = DesktopEntryCache::default();
		insert_entry(&mut cache, entry("Vesktop", "vesktop", "vesktop %U"));
		let result = cache.lookup(
			"/usr/lib/electron40/electron /usr/lib/vesktop/app.asar env \
			 ELECTRON_OZONE_PLATFORM_HINT=auto --relaunch",
			"electron",
		);
		assert!(result.is_some());
		assert_eq!(result.unwrap().icon_name, "vesktop");
	}

	#[test]
	fn test_lookup_name_prefix() {
		let mut cache = DesktopEntryCache::default();
		insert_entry(&mut cache, entry("Kate", "kate", "kate %U"));
		let result = cache.lookup("", "kate-server");
		assert!(result.is_some());
		assert_eq!(result.unwrap().icon_name, "kate");
	}

	#[test]
	fn test_lookup_flatpak_app_id() {
		let mut cache = DesktopEntryCache::default();
		insert_entry(
			&mut cache,
			entry(
				"Signal",
				"org.signal.Signal",
				"/usr/bin/flatpak run --command=signal-desktop \
				 org.signal.Signal %U",
			),
		);
		let result = cache.lookup(
			"/usr/bin/flatpak run --branch=stable org.signal.Signal %U",
			"flatpak",
		);
		assert!(result.is_some());
		assert_eq!(result.unwrap().icon_name, "org.signal.Signal");
	}

	#[test]
	fn test_lookup_longest_token_wins() {
		let mut cache = DesktopEntryCache::default();
		insert_entry(&mut cache, entry("Vesktop", "vesktop", "vesktop %U"));
		insert_entry(
			&mut cache,
			entry("Generic", "env", "env DESKTOPINTEGRATION=1 AyuGram %U"),
		);
		let result = cache.lookup(
			"/usr/lib/electron40/electron /usr/lib/vesktop/app.asar",
			"electron",
		);
		assert!(result.is_some());
		assert_eq!(result.unwrap().icon_name, "vesktop");
	}

	#[test]
	fn test_lookup_none() {
		let cache = DesktopEntryCache::default();
		let result = cache.lookup("", "unknown-binary");
		assert!(result.is_none());
	}

	#[test]
	fn test_lookup_zed_via_name_token() {
		let mut cache = DesktopEntryCache::default();
		insert_entry(&mut cache, entry("Zed", "zed", "zeditor %U"));
		let result = cache.lookup(
			"/usr/lib/zed/zed-editor zed-cli:///tmp/socket",
			"zed-editor",
		);
		assert!(result.is_some());
		assert_eq!(result.unwrap().icon_name, "zed");
	}

	#[test]
	fn test_lookup_bare_flag_arg_never_matches() {
		// A bare word in the cmdline (here `vesktop` as a --search flag
		// value on gpuitop's own command line) must not resolve to the
		// vesktop desktop entry — only path components and dotted app-ids
		// count. Regression: gpuitop --search vesktop showed the vesktop
		// icon.
		let mut cache = DesktopEntryCache::default();
		insert_entry(&mut cache, entry("Vesktop", "vesktop", "vesktop %U"));
		let result = cache.lookup(
			"/home/user/gpuitop/target/debug/gpuitop --app-id x --search \
			 vesktop",
			"gpuitop",
		);
		assert!(result.is_none());
	}

	#[test]
	fn test_lookup_path_component_matches() {
		// The electron helper case: the app path in the cmdline is a path
		// component, so it still matches.
		let mut cache = DesktopEntryCache::default();
		insert_entry(&mut cache, entry("Vesktop", "vesktop", "vesktop %U"));
		let result = cache.lookup(
			"/usr/lib/electron40/electron /usr/lib/vesktop/app.asar",
			"electron",
		);
		assert!(result.is_some());
		assert_eq!(result.unwrap().icon_name, "vesktop");
	}

	#[test]
	fn test_resolve_icon_path_empty() {
		assert_eq!(resolve_icon_path(""), None);
	}

	#[test]
	fn test_resolve_icon_path_absolute_missing() {
		assert_eq!(resolve_icon_path("/nonexistent/icon.png"), None);
	}

	#[test]
	fn test_resolve_icon_path_uncached() {
		// Cache-only: an unwarmed name resolves to None, never triggering a
		// synchronous linicon lookup on the render path.
		assert_eq!(resolve_icon_path("zzz_uncached_name_12345"), None);
	}

	#[test]
	fn test_warm_icon_paths_caches_missing_as_none() {
		// Warming an unknown name caches the miss; a second resolve is a hit
		// on a stored None rather than a repeated lookup.
		warm_icon_paths(&["zzz_warm_unknown_67890".to_string()]);
		assert_eq!(resolve_icon_path("zzz_warm_unknown_67890"), None);
	}
}
