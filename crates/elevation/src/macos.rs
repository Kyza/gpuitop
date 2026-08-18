pub fn is_elevated() -> bool {
	unsafe { libc::geteuid() == 0 }
}

pub fn relaunch_elevated() -> Result<(), String> {
	Err("relaunching as elevated is not supported on macOS".into())
}
