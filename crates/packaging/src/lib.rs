pub fn desktop(target_os: &str) -> Option<String> {
	match target_os {
		"linux" => Some(linux::desktop()),
		_ => None,
	}
}

#[path = "linux.rs"]
mod linux;
