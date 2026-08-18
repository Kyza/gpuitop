#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::{is_elevated, relaunch_elevated};
#[cfg(target_os = "macos")]
pub use macos::{is_elevated, relaunch_elevated};
#[cfg(target_os = "windows")]
pub use windows::{is_elevated, relaunch_elevated};
