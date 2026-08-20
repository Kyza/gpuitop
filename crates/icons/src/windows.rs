use std::path::PathBuf;

use super::DesktopEntryCache;

pub fn load_cache() -> DesktopEntryCache {
	DesktopEntryCache::default()
}

pub fn resolve_icon_path(_icon_name: &str) -> Option<PathBuf> {
	None
}

pub fn warm_icon_paths(_names: &[String]) {}
