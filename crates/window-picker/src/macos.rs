use super::{PickedWindow, Platform, WindowPickerInterface};

impl_interface! {
	fn is_window_picker_available() -> bool {
		false
	}

	fn pick_window() -> Option<PickedWindow> {
		None
	}
}
