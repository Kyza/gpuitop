#[path = "linux/wayland.rs"]
mod wayland;
#[path = "linux/x11.rs"]
mod x11;

use super::{PickedWindow, Platform, WindowPickerInterface};

impl_interface! {
	fn is_window_picker_available() -> bool {
		std::env::var("WAYLAND_DISPLAY").is_ok()
	}

	fn pick_window() -> Option<PickedWindow> {
		match std::env::var("WAYLAND_DISPLAY") {
			Ok(_) => wayland::pick_window(),
			Err(_) => x11::pick_window(),
		}
	}
}
