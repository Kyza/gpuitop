#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub enum PickedWindow {
	AppId(String),
	Pid(i32),
}

#[cfg(target_os = "linux")]
pub use linux::{is_window_picker_available, pick_window};
#[cfg(target_os = "macos")]
pub use macos::{is_window_picker_available, pick_window};
#[cfg(target_os = "windows")]
pub use windows::{is_window_picker_available, pick_window};
