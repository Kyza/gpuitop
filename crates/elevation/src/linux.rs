use std::process::{Command, Stdio};

pub fn is_elevated() -> bool {
	unsafe { libc::geteuid() == 0 }
}

pub fn relaunch_elevated() -> Result<(), String> {
	let exe = std::env::current_exe().map_err(|e| e.to_string())?;
	let args: Vec<String> = std::env::args().skip(1).collect();
	let cwd = std::env::current_dir().map_err(|e| e.to_string())?;

	let mut cmd = Command::new("pkexec");
	cmd.arg(&exe)
		.args(&args)
		.current_dir(cwd)
		.stdin(Stdio::null());
	for var in [
		"DISPLAY",
		"XAUTHORITY",
		"WAYLAND_DISPLAY",
		"XDG_RUNTIME_DIR",
		"XDG_SESSION_TYPE",
		"XDG_CURRENT_DESKTOP",
		"XDG_SESSION_DESKTOP",
		"DBUS_SESSION_BUS_ADDRESS",
		"LANG",
	] {
		if let Ok(v) = std::env::var(var) {
			cmd.env(var, v);
		}
	}

	cmd.spawn().map_err(|e| e.to_string())?;
	Ok(())
}
