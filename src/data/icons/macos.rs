use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct DesktopEntry {
	pub name: String,
	pub icon_name: String,
	pub exec: String,
}

#[derive(Debug, Clone, Default)]
pub struct DesktopEntryCache;

impl DesktopEntryCache {
	pub fn load() -> Self {
		Self
	}

	pub fn lookup(
		&self,
		_command: impl AsRef<str>,
		_name: impl AsRef<str>,
	) -> Option<&DesktopEntry> {
		None
	}
}

pub fn resolve_icon_path(_icon_name: &str) -> Option<PathBuf> {
	None
}
