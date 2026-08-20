use procfs::process::Process;

pub fn allowed_cpus(pid: i32) -> Result<Vec<usize>, String> {
	let mut set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
	let ret = unsafe {
		libc::sched_getaffinity(
			pid,
			std::mem::size_of::<libc::cpu_set_t>(),
			&mut set,
		)
	};
	if ret != 0 {
		return Err(std::io::Error::last_os_error().to_string());
	}
	let mut cpus = Vec::new();
	for cpu in 0..libc::CPU_SETSIZE as usize {
		if unsafe { libc::CPU_ISSET(cpu, &set) } {
			cpus.push(cpu);
		}
	}
	Ok(cpus)
}

pub fn set_affinity(pid: i32, cpus: &[usize]) -> Result<(), String> {
	if cpus.is_empty() {
		return Err("select at least one CPU".into());
	}
	let set = build_cpu_set(cpus)?;
	let tasks = Process::new(pid)
		.map_err(|e| e.to_string())?
		.tasks()
		.map_err(|e| e.to_string())?;
	let mut applied = 0;
	for task in tasks {
		let task = task.map_err(|e| e.to_string())?;
		let ret = unsafe {
			libc::sched_setaffinity(
				task.tid,
				std::mem::size_of::<libc::cpu_set_t>(),
				&set,
			)
		};
		if ret != 0 {
			let err = std::io::Error::last_os_error();
			if err.raw_os_error() == Some(libc::ESRCH) {
				continue;
			}
			return Err(err.to_string());
		}
		applied += 1;
	}
	if applied == 0 {
		return Err(format!("no live threads found for pid {pid}"));
	}
	Ok(())
}

fn build_cpu_set(cpus: &[usize]) -> Result<libc::cpu_set_t, String> {
	if cpus.iter().any(|&c| c >= libc::CPU_SETSIZE as usize) {
		return Err(format!(
			"CPU index exceeds limit of {}",
			libc::CPU_SETSIZE
		));
	}
	let mut set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
	unsafe {
		libc::CPU_ZERO(&mut set);
		for &cpu in cpus {
			libc::CPU_SET(cpu, &mut set);
		}
	}
	Ok(set)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn mask_sets_exactly_the_requested_cpus() {
		let set = build_cpu_set(&[0, 3, 7, 64]).unwrap();
		for cpu in [0, 3, 7, 64] {
			assert!(unsafe { libc::CPU_ISSET(cpu, &set) });
		}
		for cpu in [1, 2, 4, 63, 65] {
			assert!(!unsafe { libc::CPU_ISSET(cpu, &set) });
		}
	}

	#[test]
	fn mask_rejects_cpu_past_limit() {
		assert!(build_cpu_set(&[libc::CPU_SETSIZE as usize]).is_err());
	}
}
