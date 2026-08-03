use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Default)]
pub struct ProcessDetails {
	pub exists: bool,
	pub exe_path: String,
	pub cwd_path: String,
	pub name: String,
	pub state: char,
	pub ppid: i32,
	pub user_name: String,
	pub uid: u32,
	pub gid: u32,
	pub threads: u32,
	pub vm_peak: u64,
	pub vm_size: u64,
	pub vm_rss: u64,
	pub vm_data: u64,
	pub vm_stk: u64,
	pub vm_exe: u64,
	pub vm_lib: u64,
	pub vm_swap: u64,
	pub io_read_bytes: u64,
	pub io_write_bytes: u64,
	pub io_read_syscalls: u64,
	pub io_write_syscalls: u64,
	pub limits: Vec<(String, String)>,
	pub file_descriptors: Vec<(String, String)>,
	pub environment: Vec<(String, String)>,
}

fn read_string(path: &str) -> Option<String> {
	fs::read_to_string(path).ok()
}

fn read_link(path: &str) -> String {
	fs::read_link(path)
		.map(|p| p.to_string_lossy().to_string())
		.unwrap_or_default()
}

fn parse_kv(data: &str) -> HashMap<String, String> {
	let mut map = HashMap::new();
	for line in data.lines() {
		if let Some((k, v)) = line.split_once(':') {
			map.insert(k.trim().to_string(), v.trim().to_string());
		}
	}
	map
}

pub fn collect(pid: i32) -> ProcessDetails {
	let base = format!("/proc/{pid}");

	if !fs::metadata(&base).is_ok() {
		return ProcessDetails::default();
	}

	let status_map = read_string(&format!("{base}/status"))
		.map(|s| parse_kv(&s))
		.unwrap_or_default();
	let io_map = read_string(&format!("{base}/io"))
		.map(|s| parse_kv(&s))
		.unwrap_or_default();

	let exe_path = read_link(&format!("{base}/exe"));
	let cwd_path = read_link(&format!("{base}/cwd"));

	let name = status_map.get("Name").cloned().unwrap_or_default();
	let state = status_map
		.get("State")
		.and_then(|v| v.chars().next())
		.unwrap_or('?');
	let ppid: i32 = status_map
		.get("PPid")
		.and_then(|v| v.trim().parse().ok())
		.unwrap_or(0);
	let uid: u32 = status_map
		.get("Uid")
		.and_then(|v| v.split_whitespace().next())
		.and_then(|s| s.parse().ok())
		.unwrap_or(0);
	let gid: u32 = status_map
		.get("Gid")
		.and_then(|v| v.split_whitespace().next())
		.and_then(|s| s.parse().ok())
		.unwrap_or(0);
	let threads: u32 = status_map
		.get("Threads")
		.and_then(|v| v.trim().parse().ok())
		.unwrap_or(0);

	let vm_peak = parse_kb(&status_map, "VmPeak");
	let vm_size = parse_kb(&status_map, "VmSize");
	let vm_rss = parse_kb(&status_map, "VmRSS");
	let vm_data = parse_kb(&status_map, "VmData");
	let vm_stk = parse_kb(&status_map, "VmStk");
	let vm_exe = parse_kb(&status_map, "VmExe");
	let vm_lib = parse_kb(&status_map, "VmLib");
	let vm_swap = parse_kb(&status_map, "VmSwap");

	let io_read_bytes: u64 = io_map
		.get("read_bytes")
		.and_then(|v| v.parse().ok())
		.unwrap_or(0);
	let io_write_bytes: u64 = io_map
		.get("write_bytes")
		.and_then(|v| v.parse().ok())
		.unwrap_or(0);
	let io_read_syscalls: u64 = io_map
		.get("syscr")
		.and_then(|v| v.parse().ok())
		.unwrap_or(0);
	let io_write_syscalls: u64 = io_map
		.get("syscw")
		.and_then(|v| v.parse().ok())
		.unwrap_or(0);

	let user_name = users::get_user_by_uid(uid)
		.map(|u| u.name().to_string_lossy().to_string())
		.unwrap_or_else(|| uid.to_string());

	let limits = read_limits(pid);
	let file_descriptors = read_fds(pid);
	let environment = read_environ(pid);

	ProcessDetails {
		exists: true,
		exe_path,
		cwd_path,
		name,
		state,
		ppid,
		user_name,
		uid,
		gid,
		threads,
		vm_peak,
		vm_size,
		vm_rss,
		vm_data,
		vm_stk,
		vm_exe,
		vm_lib,
		vm_swap,
		io_read_bytes,
		io_write_bytes,
		io_read_syscalls,
		io_write_syscalls,
		limits,
		file_descriptors,
		environment,
	}
}

fn parse_kb(map: &HashMap<String, String>, key: &str) -> u64 {
	map.get(key)
		.and_then(|v| v.split_whitespace().next())
		.and_then(|s| s.parse::<u64>().ok())
		.unwrap_or(0)
		* 1024
}

fn read_limits(pid: i32) -> Vec<(String, String)> {
	let path = format!("/proc/{pid}/limits");
	let data = match read_string(&path) {
		Some(d) => d,
		None => return Vec::new(),
	};
	let mut result = Vec::new();
	for line in data.lines().skip(1) {
		let cols: Vec<&str> = line.split_whitespace().collect();
		if cols.len() >= 4 {
			let limit_name = cols[0].to_string();
			let soft = cols[1].to_string();
			let hard = cols[2].to_string();
			result.push((limit_name, format!("{soft} / {hard}")));
		}
	}
	result
}

fn read_fds(pid: i32) -> Vec<(String, String)> {
	let dir_path = format!("/proc/{pid}/fd");
	let dir = match fs::read_dir(&dir_path) {
		Ok(d) => d,
		Err(_) => return Vec::new(),
	};
	let mut result = Vec::new();
	for entry in dir.flatten() {
		let fd_num = entry.file_name().to_string_lossy().to_string();
		let target = fs::read_link(entry.path())
			.map(|p| p.to_string_lossy().to_string())
			.unwrap_or_default();
		result.push((fd_num, target));
	}
	result.sort_by(|a, b| {
		a.0.parse::<u32>()
			.unwrap_or(0)
			.cmp(&b.0.parse::<u32>().unwrap_or(0))
	});
	result
}

fn read_environ(pid: i32) -> Vec<(String, String)> {
	let path = format!("/proc/{pid}/environ");
	let data = match fs::read(&path) {
		Ok(d) => d,
		Err(_) => return Vec::new(),
	};
	let mut result = Vec::new();
	for chunk in data.split(|&b| b == 0) {
		if chunk.is_empty() {
			continue;
		}
		let s = String::from_utf8_lossy(chunk);
		if let Some((k, v)) = s.split_once('=') {
			result.push((k.to_string(), v.to_string()));
		}
	}
	result.sort_by(|a, b| a.0.cmp(&b.0));
	result
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_collect_current_process() {
		let details = collect(std::process::id() as i32);
		assert!(details.exists);
		assert!(!details.name.is_empty());
		assert!(details.state != '?');
		assert!(details.vm_rss > 0);
		assert!(details.threads > 0);
		assert!(!details.exe_path.is_empty());
	}

	#[test]
	fn test_collect_nonexistent_process() {
		let details = collect(99999999);
		assert!(!details.exists);
	}

	#[test]
	fn test_collect_init_process() {
		let details = collect(1);
		assert!(details.exists);
	}
}
