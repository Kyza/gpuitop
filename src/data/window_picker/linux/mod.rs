mod wayland;
mod x11;

/// Wayland is available, but not x11.
pub fn is_window_picker_available() -> bool {
	std::env::var("WAYLAND_DISPLAY").is_ok()
}

pub fn pick_window() -> Option<super::PickedWindow> {
	match std::env::var("WAYLAND_DISPLAY") {
		Ok(_) => wayland::pick_window(),
		Err(_) => x11::pick_window(),
	}
}
