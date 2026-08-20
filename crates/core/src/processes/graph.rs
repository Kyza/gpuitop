use crate::model::{ProcessSnapshot, SystemSnapshot};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// The process tree derived from one snapshot: parent/child structure, full
/// subtree counts, and ancestry walks. Built once per snapshot and immutable.
/// All walks are cycle-guarded so a corrupted ppid chain can never loop.
pub struct ProcessGraph {
	snapshot: Rc<SystemSnapshot>,
	pid_to_idx: HashMap<i32, usize>,
	children_by_ppid: HashMap<i32, Vec<i32>>,
	subtree_counts: HashMap<i32, usize>,
}

impl ProcessGraph {
	pub fn new(snapshot: Rc<SystemSnapshot>) -> Self {
		let procs = &snapshot.processes;
		let mut pid_to_idx = HashMap::with_capacity(procs.len());
		let mut children_by_ppid: HashMap<i32, Vec<i32>> = HashMap::new();
		for (i, p) in procs.iter().enumerate() {
			pid_to_idx.insert(p.pid, i);
			children_by_ppid.entry(p.ppid).or_default().push(p.pid);
		}
		for children in children_by_ppid.values_mut() {
			children.sort_unstable();
		}
		let subtree_counts = Self::compute_subtree_counts(&children_by_ppid);
		Self {
			snapshot,
			pid_to_idx,
			children_by_ppid,
			subtree_counts,
		}
	}

	fn compute_subtree_counts(
		children_by_ppid: &HashMap<i32, Vec<i32>>,
	) -> HashMap<i32, usize> {
		fn count(
			pid: i32,
			children_by_ppid: &HashMap<i32, Vec<i32>>,
			memo: &mut HashMap<i32, usize>,
			in_progress: &mut HashSet<i32>,
		) -> usize {
			if let Some(&c) = memo.get(&pid) {
				return c;
			}
			if !in_progress.insert(pid) {
				return 0;
			}
			let c = children_by_ppid
				.get(&pid)
				.map(|kids| {
					kids.iter()
						.map(|&k| {
							1 + count(k, children_by_ppid, memo, in_progress)
						})
						.sum()
				})
				.unwrap_or(0);
			in_progress.remove(&pid);
			memo.insert(pid, c);
			c
		}
		let mut memo = HashMap::new();
		let mut in_progress = HashSet::new();
		let parents: Vec<i32> = children_by_ppid.keys().copied().collect();
		for pid in parents {
			count(pid, children_by_ppid, &mut memo, &mut in_progress);
		}
		memo
	}

	pub fn children_of(&self, pid: i32) -> &[i32] {
		self.children_by_ppid
			.get(&pid)
			.map_or(&[], |c| c.as_slice())
	}

	pub fn is_descendant_of(&self, child_pid: i32, ancestor: i32) -> bool {
		child_pid != ancestor
			&& self.ancestors_of(child_pid).contains(&ancestor)
	}

	pub fn descendants_of(&self, pid: i32) -> Vec<i32> {
		let mut out = Vec::new();
		let mut stack = vec![pid];
		let mut seen = HashSet::new();
		seen.insert(pid);
		while let Some(cur) = stack.pop() {
			for &k in self.children_of(cur) {
				if seen.insert(k) {
					out.push(k);
					stack.push(k);
				}
			}
		}
		out
	}

	pub fn subtree_count_of(&self, pid: i32) -> usize {
		self.subtree_counts.get(&pid).copied().unwrap_or(0)
	}

	pub fn subtree_counts(&self) -> &HashMap<i32, usize> {
		&self.subtree_counts
	}

	fn ancestors_of(&self, pid: i32) -> Vec<i32> {
		let mut out = Vec::new();
		let mut current = pid;
		let mut seen = HashSet::new();
		seen.insert(pid);
		while let Some(&idx) = self.pid_to_idx.get(&current) {
			let ppid = self.snapshot.processes[idx].ppid;
			if ppid == 0 || ppid == current || !seen.insert(ppid) {
				break;
			}
			out.push(ppid);
			current = ppid;
		}
		out.reverse();
		out
	}

	pub fn ancestor_chain_of(&self, target_pid: i32) -> Vec<ProcessSnapshot> {
		let Some(&target_idx) = self.pid_to_idx.get(&target_pid) else {
			return Vec::new();
		};
		let mut chain: Vec<ProcessSnapshot> = self
			.ancestors_of(target_pid)
			.iter()
			.map(|&pid| {
				self.snapshot.processes[self.pid_to_idx[&pid]].clone()
			})
			.collect();
		chain.push(self.snapshot.processes[target_idx].clone());
		chain
	}

	pub fn with_ancestors(&self, matched: &HashSet<i32>) -> HashSet<i32> {
		let mut display = matched.clone();
		for &mpid in matched {
			display.extend(self.ancestors_of(mpid));
		}
		display
	}

	pub fn ancestors_to_expand(
		&self,
		matched: &HashSet<i32>,
	) -> HashSet<i32> {
		let mut expand = HashSet::new();
		for &mpid in matched {
			expand.extend(self.ancestors_of(mpid));
		}
		expand
	}

	/// Descendant counts restricted to a display set: how many of each pid's
	/// descendants are actually shown. Drives the tree's `(+shown/total)` badge.
	pub fn display_subtree_counts(
		&self,
		display: &HashSet<i32>,
	) -> HashMap<i32, usize> {
		fn count(
			pid: i32,
			graph: &ProcessGraph,
			display: &HashSet<i32>,
			memo: &mut HashMap<i32, usize>,
		) -> usize {
			if let Some(&c) = memo.get(&pid) {
				return c;
			}
			let c = graph
				.children_of(pid)
				.iter()
				.filter(|c| display.contains(c))
				.map(|&c| 1 + count(c, graph, display, memo))
				.sum();
			memo.insert(pid, c);
			c
		}
		let mut memo = HashMap::new();
		for &pid in display {
			count(pid, self, display, &mut memo);
		}
		memo
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::model::{MemoryInfo, VramUsage};

	fn make_snapshot(procs: Vec<ProcessSnapshot>) -> Rc<SystemSnapshot> {
		Rc::new(SystemSnapshot {
			processes: procs,
			cpu: crate::model::CpuInfo {
				cores: vec![],
				overall_percent: 0.0,
				model_name: String::new(),
				temperature: 0.0,
			},
			memory: MemoryInfo {
				total: 0,
				used: 0,
				available: 0,
				cached: 0,
				swap_used: 0,
			},
			disks: vec![],
			networks: vec![],
			timestamp: std::time::Instant::now(),
			gpu_backends: vec![],
			gpu_devices: vec![],
			gpu_polling_enabled: false,
		})
	}

	fn make_proc(pid: i32, ppid: i32) -> ProcessSnapshot {
		ProcessSnapshot {
			pid,
			ppid,
			name: format!("proc-{pid}"),
			user: "root".into(),
			state: 'S',
			command: String::new(),
			cgroup: String::new(),
			cpu_percent: 0.0,
			mem_percent: 0.0,
			mem_rss: 0,
			vram: VramUsage::default(),
			disk_read_bytes_per_sec: 0.0,
			disk_write_bytes_per_sec: 0.0,
			is_gui: false,
			is_kthread: false,
			is_owned_by_current_user: false,
			is_electron: false,
			electron_app_name: None,
			icon_name: None,
			has_children: false,
		}
	}

	fn graph_of(procs: Vec<ProcessSnapshot>) -> ProcessGraph {
		ProcessGraph::new(make_snapshot(procs))
	}

	// ── is_descendant_of ─────────────────────────────────────

	#[test]
	fn is_descendant_of_direct_child() {
		let g = graph_of(vec![make_proc(1, 0), make_proc(10, 1)]);
		assert!(g.is_descendant_of(10, 1));
	}

	#[test]
	fn is_descendant_of_grandchild() {
		let g = graph_of(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(100, 10),
		]);
		assert!(g.is_descendant_of(100, 1));
	}

	#[test]
	fn is_descendant_of_not_descendant() {
		let g = graph_of(vec![make_proc(1, 0), make_proc(10, 2)]);
		assert!(!g.is_descendant_of(10, 1));
	}

	#[test]
	fn is_descendant_of_self_is_false() {
		let g = graph_of(vec![make_proc(1, 0)]);
		assert!(!g.is_descendant_of(1, 1));
	}

	#[test]
	fn is_descendant_of_unknown_pid_is_false() {
		let g = graph_of(vec![make_proc(1, 0)]);
		assert!(!g.is_descendant_of(999, 1));
	}

	#[test]
	fn is_descendant_of_cycle_terminates() {
		let g = graph_of(vec![make_proc(10, 20), make_proc(20, 10)]);
		assert!(g.is_descendant_of(10, 20));
		assert!(!g.is_descendant_of(10, 99));
	}

	// ── descendants_of ───────────────────────────────────────

	#[test]
	fn descendants_of_subtree() {
		let g = graph_of(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(20, 10),
			make_proc(30, 1),
		]);
		let mut d = g.descendants_of(1);
		d.sort_unstable();
		assert_eq!(d, vec![10, 20, 30]);
	}

	#[test]
	fn descendants_of_leaf_is_empty() {
		let g = graph_of(vec![make_proc(1, 0), make_proc(10, 1)]);
		assert!(g.descendants_of(10).is_empty());
	}

	#[test]
	fn descendants_of_cycle_terminates() {
		let g = graph_of(vec![make_proc(10, 20), make_proc(20, 10)]);
		assert_eq!(g.descendants_of(10).len(), 1);
	}

	// ── subtree counts ───────────────────────────────────────

	#[test]
	fn subtree_counts_parent_with_children() {
		let g = graph_of(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(20, 1),
			make_proc(30, 1),
		]);
		assert_eq!(g.subtree_count_of(1), 3);
	}

	#[test]
	fn subtree_counts_leaf_is_zero() {
		let g = graph_of(vec![make_proc(1, 0), make_proc(10, 1)]);
		assert_eq!(g.subtree_count_of(10), 0);
	}

	#[test]
	fn subtree_counts_nested() {
		let g = graph_of(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(20, 10),
			make_proc(30, 20),
		]);
		assert_eq!(g.subtree_count_of(1), 3);
		assert_eq!(g.subtree_count_of(10), 2);
	}

	#[test]
	fn subtree_counts_cycle_terminates() {
		let g = graph_of(vec![make_proc(10, 20), make_proc(20, 10)]);
		let count = g.subtree_count_of(10);
		assert!(count <= 2);
	}

	// ── ancestor_chain_of ────────────────────────────────────

	#[test]
	fn ancestor_chain_leaf_to_root() {
		let g = graph_of(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(100, 10),
		]);
		let chain = g.ancestor_chain_of(100);
		let pids: Vec<i32> = chain.iter().map(|p| p.pid).collect();
		assert_eq!(pids, vec![1, 10, 100]);
	}

	#[test]
	fn ancestor_chain_root_is_itself() {
		let g = graph_of(vec![make_proc(1, 0), make_proc(10, 1)]);
		let chain = g.ancestor_chain_of(1);
		let pids: Vec<i32> = chain.iter().map(|p| p.pid).collect();
		assert_eq!(pids, vec![1]);
	}

	#[test]
	fn ancestor_chain_unknown_pid_empty() {
		let g = graph_of(vec![make_proc(1, 0)]);
		assert!(g.ancestor_chain_of(999).is_empty());
	}

	#[test]
	fn ancestor_chain_cycle_terminates() {
		let g = graph_of(vec![make_proc(10, 20), make_proc(20, 10)]);
		assert_eq!(g.ancestor_chain_of(10).len(), 2);
	}

	// ── with_ancestors ───────────────────────────────────────

	#[test]
	fn with_ancestors_single_match() {
		// 1 -> 100 -> 200
		let g = graph_of(vec![
			make_proc(200, 100),
			make_proc(100, 1),
			make_proc(1, 0),
		]);
		let matched: HashSet<i32> = [200].iter().copied().collect();
		let display = g.with_ancestors(&matched);
		assert!(display.contains(&200));
		assert!(display.contains(&100));
		assert!(display.contains(&1));
		assert_eq!(display.len(), 3);
	}

	#[test]
	fn with_ancestors_sibling_not_included() {
		// 1 -> 100 -> 200 (match)
		//              -> 201 (sibling, not match)
		let g = graph_of(vec![
			make_proc(200, 100),
			make_proc(201, 100),
			make_proc(100, 1),
			make_proc(1, 0),
		]);
		let matched: HashSet<i32> = [200].iter().copied().collect();
		let display = g.with_ancestors(&matched);
		assert!(display.contains(&200));
		assert!(display.contains(&100));
		assert!(display.contains(&1));
		assert!(!display.contains(&201));
		assert_eq!(display.len(), 3);
	}

	#[test]
	fn with_ancestors_cycle_terminates() {
		let g = graph_of(vec![make_proc(10, 20), make_proc(20, 10)]);
		let matched: HashSet<i32> = [10].iter().copied().collect();
		let display = g.with_ancestors(&matched);
		assert!(display.contains(&10));
	}

	// ── ancestors_to_expand ──────────────────────────────────

	#[test]
	fn ancestors_to_expand_chain() {
		let g = graph_of(vec![
			make_proc(200, 100),
			make_proc(100, 1),
			make_proc(1, 0),
		]);
		let matched: HashSet<i32> = [200].iter().copied().collect();
		let expand = g.ancestors_to_expand(&matched);
		assert!(expand.contains(&100));
		assert!(expand.contains(&1));
		assert!(!expand.contains(&200));
		assert_eq!(expand.len(), 2);
	}

	// ── display_subtree_counts ───────────────────────────────

	#[test]
	fn display_counts_respect_display_set() {
		// 1 -> 10 -> 20, 30 ; 21 is a sibling of 20
		let g = graph_of(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(20, 10),
			make_proc(21, 10),
			make_proc(30, 10),
		]);
		let display: HashSet<i32> = [1, 10, 20, 30].iter().copied().collect();
		let counts = g.display_subtree_counts(&display);
		assert_eq!(counts.get(&10).copied().unwrap_or(0), 2);
		assert_eq!(counts.get(&1).copied().unwrap_or(0), 3);
	}
}
