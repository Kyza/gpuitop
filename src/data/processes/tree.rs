use crate::data::model::ProcessInfo;
use std::collections::{HashMap, HashSet};

pub fn with_ancestors(
	matched: &HashSet<i32>,
	pid_to_ppid: &HashMap<i32, i32>,
) -> HashSet<i32> {
	let mut display = matched.clone();
	for &mpid in matched {
		let mut current = pid_to_ppid.get(&mpid).copied().unwrap_or(0);
		while current != 0 && current != mpid {
			display.insert(current);
			let next = pid_to_ppid.get(&current).copied().unwrap_or(0);
			if next == 0 || next == current {
				break;
			}
			current = next;
		}
	}
	display
}

pub fn ancestors_to_expand(
	matched: &HashSet<i32>,
	pid_lookup: &HashMap<i32, ProcessInfo>,
) -> HashSet<i32> {
	let mut expand = HashSet::new();
	for &mpid in matched {
		let mut current = pid_lookup.get(&mpid).map(|p| p.ppid).unwrap_or(0);
		while let Some(proc) = pid_lookup.get(&current) {
			expand.insert(current);
			let ppid = proc.ppid;
			if ppid == 0 || ppid == current {
				break;
			}
			current = ppid;
		}
	}
	expand
}

#[cfg(test)]
mod tests {
	use super::*;

	fn make_proc(pid: i32, ppid: i32) -> ProcessInfo {
		ProcessInfo {
			pid,
			ppid,
			name: String::new(),
			user: String::new(),
			state: 'S',
			command: String::new(),
			cgroup: String::new(),
			cpu_percent: 0.0,
			mem_percent: 0.0,
			mem_rss: 0,
			vram_bytes: None,
			disk_read_bytes_per_sec: 0.0,
			disk_write_bytes_per_sec: 0.0,
			is_gui: false,
			is_kthread: false,
			is_owned_by_current_user: false,
			is_electron: false,
			electron_app_name: None,
			children: vec![],
			has_children: false,
		}
	}

	#[test]
	fn test_with_ancestors_single_match() {
		// 1 -> 100 -> 200
		let mut map = HashMap::new();
		map.insert(200, 100);
		map.insert(100, 1);
		map.insert(1, 0);

		let matched: HashSet<i32> = [200].iter().copied().collect();
		let display = with_ancestors(&matched, &map);

		assert!(display.contains(&200));
		assert!(display.contains(&100));
		assert!(display.contains(&1));
		assert_eq!(display.len(), 3);
	}

	#[test]
	fn test_with_ancestors_sibling_not_included() {
		// 1 -> 100 -> 200 (match)
		//              -> 201 (sibling, not match)
		let mut map = HashMap::new();
		map.insert(200, 100);
		map.insert(201, 100);
		map.insert(100, 1);
		map.insert(1, 0);

		let matched: HashSet<i32> = [200].iter().copied().collect();
		let display = with_ancestors(&matched, &map);

		assert!(display.contains(&200));
		assert!(display.contains(&100));
		assert!(display.contains(&1));
		assert!(!display.contains(&201));
		assert_eq!(display.len(), 3);
	}

	#[test]
	fn test_ancestors_to_expand() {
		let pid_lookup: HashMap<i32, ProcessInfo> = [
			(1, make_proc(1, 0)),
			(100, make_proc(100, 1)),
			(200, make_proc(200, 100)),
		]
		.iter()
		.cloned()
		.collect();

		let matched: HashSet<i32> = [200].iter().copied().collect();
		let expand = ancestors_to_expand(&matched, &pid_lookup);

		assert!(expand.contains(&100));
		assert!(expand.contains(&1));
		assert!(!expand.contains(&200));
		assert_eq!(expand.len(), 2);
	}
}
