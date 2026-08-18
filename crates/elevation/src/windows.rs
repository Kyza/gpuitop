pub fn is_elevated() -> bool {
	false
}

pub fn relaunch_elevated() -> Result<(), String> {
	Err("relaunching as elevated is not supported on Windows".into())
}
