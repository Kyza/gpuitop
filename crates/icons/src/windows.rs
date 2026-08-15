use std::path::PathBuf;

use super::{DesktopEntryCache, IconsInterface, Platform};

impl_interface! {
	fn load_cache() -> DesktopEntryCache {
		DesktopEntryCache::default()
	}

	fn resolve_icon_path(_icon_name: &str) -> Option<PathBuf> {
		None
	}
}
