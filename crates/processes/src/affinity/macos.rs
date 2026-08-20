pub fn allowed_cpus(_pid: i32) -> Result<Vec<usize>, String> {
	Err("CPU affinity is not supported on this platform".into())
}

pub fn set_affinity(_pid: i32, _cpus: &[usize]) -> Result<(), String> {
	Err("CPU affinity is not supported on this platform".into())
}
