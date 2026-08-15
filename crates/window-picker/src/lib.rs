use oswap::{define_interface, define_platforms};

pub const GPUITOP_APP_ID: &str = "com.github.kyza.gpuitop";

pub enum PickedWindow {
	AppId(String),
	Pid(i32),
}

define_interface! { Platform, WindowPickerInterface, impl_interface,
	pub fn is_window_picker_available() -> bool;
	pub fn pick_window() -> Option<PickedWindow>;
}

define_platforms![
	{ file: "linux", cfg: target_os = "linux" },
	{ file: "macos", cfg: target_os = "macos" },
	{ file: "windows", cfg: target_os = "windows" },
];
