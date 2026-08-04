#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::get_focused_window_app_id;
#[cfg(target_os = "macos")]
pub use macos::get_focused_window_app_id;
#[cfg(target_os = "windows")]
pub use windows::get_focused_window_app_id;
