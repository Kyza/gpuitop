#[path = "linux/wayland.rs"]
mod wayland;
#[path = "linux/x11.rs"]
mod x11;

use super::PickedWindow;

pub fn is_window_picker_available() -> bool {
	std::env::var("WAYLAND_DISPLAY").is_ok()
		|| std::env::var("DISPLAY").is_ok()
}

pub fn pick_window() -> Option<PickedWindow> {
	if std::env::var("WAYLAND_DISPLAY").is_ok() {
		wayland::pick_window()
	} else {
		x11::pick_window()
	}
}
