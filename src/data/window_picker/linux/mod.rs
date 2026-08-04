#[cfg(any(feature = "runtime-compositor", feature = "wayland"))]
mod wayland;
#[cfg(any(feature = "runtime-compositor", feature = "x11"))]
mod x11;

pub fn get_focused_window_app_id() -> Option<String> {
	#[cfg(feature = "runtime-compositor")]
	{
		if std::env::var("WAYLAND_DISPLAY").is_ok() {
			return wayland::get_focused_window_app_id();
		}
		return x11::get_focused_window_app_id();
	}

	#[cfg(all(not(feature = "runtime-compositor"), feature = "wayland"))]
	{
		wayland::get_focused_window_app_id()
	}

	#[cfg(all(
		not(feature = "runtime-compositor"),
		not(feature = "wayland"),
		feature = "x11"
	))]
	{
		x11::get_focused_window_app_id()
	}

	#[cfg(all(
		not(feature = "runtime-compositor"),
		not(feature = "wayland"),
		not(feature = "x11")
	))]
	{
		None
	}
}
