use crate::data::config::ProcessesConfig;
use crate::data::fuzzy::{best_fuzzy_score, fuzzy_match};
use crate::data::model::*;
use crate::data::platform::system::{
	is_service, pids_of_runsv, pids_of_supervise_daemon, InitSystem,
};
use crate::data::state::{CumulativeResources, ViewState};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct ProcessTableDelegate {
	pub snapshot_cell: Rc<RefCell<Rc<SystemSnapshot>>>,
	pub cum_cache: Rc<RefCell<Option<HashMap<i32, CumulativeResources>>>>,
	pub pid_index: Rc<RefCell<Option<HashMap<i32, usize>>>>,
	pub view_state: Rc<RefCell<ViewState>>,
	pub column_visibility: ProcessesConfig,
	pub init_system: InitSystem,
	pub descendant_counts: Rc<RefCell<Option<HashMap<i32, usize>>>>,
}

impl ProcessTableDelegate {
	pub fn is_col_hidden(&self, col_ix: usize) -> bool {
		if col_ix == 0 {
			return false;
		}
		SortColumn::from_col_index(col_ix)
			.map(|col| !self.column_visibility.is_col_visible(col))
			.unwrap_or(false)
	}

	#[hotpath::measure]
	pub fn is_descendant_of(&self, child_pid: i32, ancestor: i32) -> bool {
		let procs = &self.snapshot_cell.borrow().processes;
		if self.pid_index.borrow().is_none() {
			let mut map = HashMap::with_capacity(procs.len());
			for (i, p) in procs.iter().enumerate() {
				map.insert(p.pid, i);
			}
			*self.pid_index.borrow_mut() = Some(map);
		}
		let idx_map = self.pid_index.borrow();
		let idx_map = idx_map.as_ref().unwrap();
		if child_pid == ancestor {
			return false;
		}
		let mut current = child_pid;
		for _ in 0..100 {
			if current == ancestor {
				return true;
			}
			if let Some(&idx) = idx_map.get(&current) {
				let p = &procs[idx];
				if p.ppid == 0 || p.ppid == current {
					return false;
				}
				current = p.ppid;
			} else {
				return false;
			}
		}
		false
	}

	#[hotpath::measure]
	pub fn compute_descendant_counts(&self) -> HashMap<i32, usize> {
		if let Some(ref cached) = *self.descendant_counts.borrow() {
			return cached.clone();
		}
		let procs = &self.snapshot_cell.borrow().processes;
		if self.pid_index.borrow().is_none() {
			let mut map = HashMap::with_capacity(procs.len());
			for (i, p) in procs.iter().enumerate() {
				map.insert(p.pid, i);
			}
			*self.pid_index.borrow_mut() = Some(map);
		}
		let idx_map = self.pid_index.borrow();
		let idx_map = idx_map.as_ref().unwrap();

		let mut counts: HashMap<i32, usize> =
			HashMap::with_capacity(procs.len());
		for p in procs {
			let mut cur = p.ppid;
			loop {
				if cur == 0 {
					break;
				}
				if let Some(&idx) = idx_map.get(&cur) {
					let parent = &procs[idx];
					*counts.entry(cur).or_insert(0) += 1;
					if parent.ppid == 0 || parent.ppid == cur {
						break;
					}
					cur = parent.ppid;
				} else {
					break;
				}
			}
		}

		*self.descendant_counts.borrow_mut() = Some(counts.clone());
		counts
	}

	pub fn count_descendants_of(&self, pid: i32) -> usize {
		if let Some(ref cached) = *self.descendant_counts.borrow() {
			return cached.get(&pid).copied().unwrap_or(0);
		}
		self.compute_descendant_counts()
			.get(&pid)
			.copied()
			.unwrap_or(0)
	}

	pub fn ancestor_chain_of(&self, target_pid: i32) -> Vec<ProcessInfo> {
		let procs = &self.snapshot_cell.borrow().processes;
		if self.pid_index.borrow().is_none() {
			let mut map = HashMap::with_capacity(procs.len());
			for (i, p) in procs.iter().enumerate() {
				map.insert(p.pid, i);
			}
			*self.pid_index.borrow_mut() = Some(map);
		}
		let idx_map = self.pid_index.borrow();
		let idx_map = idx_map.as_ref().unwrap();
		let mut chain = Vec::new();
		let mut current = target_pid;
		for _ in 0..100 {
			if let Some(&idx) = idx_map.get(&current) {
				let proc = &procs[idx];
				chain.push(proc.clone());
				if proc.ppid == 0 || proc.ppid == current {
					break;
				}
				current = proc.ppid;
			} else {
				break;
			}
		}
		chain.reverse();
		chain
	}

	#[hotpath::measure]
	pub fn compute_aggregate_cumulative_map(
		&self,
	) -> HashMap<i32, CumulativeResources> {
		if let Some(ref cached) = *self.cum_cache.borrow() {
			return cached.clone();
		}
		let procs = &self.snapshot_cell.borrow().processes;
		if procs.is_empty() {
			return HashMap::new();
		}

		let mut cum: HashMap<i32, CumulativeResources> =
			HashMap::with_capacity(procs.len());
		let mut pid_map: HashMap<i32, &ProcessInfo> =
			HashMap::with_capacity(procs.len());
		let mut pids: Vec<i32> = Vec::with_capacity(procs.len());

		for p in procs {
			pid_map.insert(p.pid, p);
			pids.push(p.pid);
			cum.insert(
				p.pid,
				CumulativeResources {
					cpu: p.cpu_percent,
					mem_rss: p.mem_rss,
					vram: p.vram_bytes,
					disk_read: p.disk_read_bytes_per_sec,
					disk_write: p.disk_write_bytes_per_sec,
				},
			);
		}

		let mut depth: HashMap<i32, usize> =
			HashMap::with_capacity(procs.len());
		for &pid in &pids {
			if depth.contains_key(&pid) {
				continue;
			}
			let mut chain = vec![pid];
			let mut cur = pid;
			loop {
				let ppid = pid_map.get(&cur).map(|p| p.ppid).unwrap_or(0);
				if ppid == 0 || ppid == cur || depth.contains_key(&ppid) {
					break;
				}
				chain.push(ppid);
				cur = ppid;
			}
			let last = *chain.last().unwrap();
			let last_ppid = pid_map.get(&last).map(|p| p.ppid).unwrap_or(0);
			let base = if last_ppid == 0 || last_ppid == last {
				0
			} else {
				depth.get(&last_ppid).copied().unwrap_or(0)
			};
			for (i, &node) in chain.iter().rev().enumerate() {
				depth.insert(node, base + i + 1);
			}
		}

		pids.sort_by_key(|pid| {
			std::cmp::Reverse(depth.get(pid).copied().unwrap_or(0))
		});

		for &pid in &pids {
			let ppid = pid_map.get(&pid).map(|p| p.ppid).unwrap_or(0);
			if ppid == 0 || ppid == pid {
				continue;
			}
			let child_cpu = cum[&pid].cpu;
			let child_mem = cum[&pid].mem_rss;
			let child_vram = cum[&pid].vram;
			let child_dr = cum[&pid].disk_read;
			let child_dw = cum[&pid].disk_write;
			if let Some(parent) = cum.get_mut(&ppid) {
				parent.cpu += child_cpu;
				parent.mem_rss += child_mem;
				parent.vram = match (parent.vram, child_vram) {
					(Some(a), Some(b)) => Some(a + b),
					(a, None) => a,
					(None, b) => b,
				};
				parent.disk_read += child_dr;
				parent.disk_write += child_dw;
			}
		}

		*self.cum_cache.borrow_mut() = Some(cum.clone());
		cum
	}

	#[hotpath::measure]
	pub fn filtered_sorted_rows(&self) -> Rc<Vec<ProcessInfo>> {
		let ts = self.snapshot_cell.borrow().timestamp;
		{
			let vs = self.view_state.borrow();
			if let Some((cached_ts, cached_gen, ref rows)) = vs.cached {
				if cached_ts == ts && cached_gen == vs.generation {
					return rows.clone();
				}
			}
		}

		let vs_ref = self.view_state.borrow();
		let search = vs_ref.search.to_lowercase();
		let filters = vs_ref.filters.clone();
		let filter_mode = vs_ref.filter_mode;
		let resource_view_mode = vs_ref.resource_view_mode;
		let sort_col = vs_ref.sort_col;
		let sort_dir = vs_ref.sort_dir;
		drop(vs_ref);

		let all = self.snapshot_cell.borrow().processes.clone();

		let mut matcher = nucleo::Matcher::new(nucleo::Config::DEFAULT);

		let mut result: Vec<ProcessInfo> = all
			.iter()
			.filter(|p| {
				if !search.is_empty() {
					if !fuzzy_match(
						&search,
						&p.name.to_lowercase(),
						&mut matcher,
					) && !p.pid.to_string().contains(&search)
						&& !fuzzy_match(
							&search,
							&p.command.to_lowercase(),
							&mut matcher,
						) && !fuzzy_match(
						&search,
						&p.electron_app_name
							.as_deref()
							.unwrap_or_default()
							.to_lowercase(),
						&mut matcher,
					) {
						return false;
					}
				}
				if filters.is_empty() {
					return true;
				}
				match filter_mode {
					FilterMode::And => {
						filters.iter().all(|f| self.proc_matches(p, f))
					}
					FilterMode::Or => {
						filters.iter().any(|f| self.proc_matches(p, f))
					}
				}
			})
			.cloned()
			.collect();

		let col = sort_col;
		let use_cum =
			col >= 5 && resource_view_mode == ResourceViewMode::Cumulative;

		let cum_map: HashMap<i32, CumulativeResources> = if use_cum {
			self.compute_aggregate_cumulative_map()
		} else {
			HashMap::new()
		};

		result.sort_by(|a, b| {
			if !search.is_empty() {
				let sa = best_fuzzy_score(&search, a, &mut matcher);
				let sb = best_fuzzy_score(&search, b, &mut matcher);
				match sb.cmp(&sa) {
					std::cmp::Ordering::Equal => {}
					other => return other,
				}
			}
			let cmp = match col {
				1 => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
				2 => a.pid.cmp(&b.pid),
				3 => a.user.to_lowercase().cmp(&b.user.to_lowercase()),
				4 => a.state.cmp(&b.state),
				5 => {
					let va = if use_cum {
						cum_map.get(&a.pid).map_or(0.0, |c| c.cpu)
					} else {
						a.cpu_percent
					};
					let vb = if use_cum {
						cum_map.get(&b.pid).map_or(0.0, |c| c.cpu)
					} else {
						b.cpu_percent
					};
					va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
				}
				6 => {
					let va = if use_cum {
						cum_map.get(&a.pid).map_or(0, |c| c.mem_rss) as f32
					} else {
						a.mem_rss as f32
					};
					let vb = if use_cum {
						cum_map.get(&b.pid).map_or(0, |c| c.mem_rss) as f32
					} else {
						b.mem_rss as f32
					};
					va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
				}
				7 => {
					let va = if use_cum {
						cum_map.get(&a.pid).and_then(|c| c.vram)
					} else {
						a.vram_bytes
					};
					let vb = if use_cum {
						cum_map.get(&b.pid).and_then(|c| c.vram)
					} else {
						b.vram_bytes
					};
					va.cmp(&vb)
				}
				8 => {
					let va = if use_cum {
						cum_map.get(&a.pid).map_or(0.0, |c| c.disk_read)
					} else {
						a.disk_read_bytes_per_sec
					};
					let vb = if use_cum {
						cum_map.get(&b.pid).map_or(0.0, |c| c.disk_read)
					} else {
						b.disk_read_bytes_per_sec
					};
					va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
				}
				9 => {
					let va = if use_cum {
						cum_map.get(&a.pid).map_or(0.0, |c| c.disk_write)
					} else {
						a.disk_write_bytes_per_sec
					};
					let vb = if use_cum {
						cum_map.get(&b.pid).map_or(0.0, |c| c.disk_write)
					} else {
						b.disk_write_bytes_per_sec
					};
					va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
				}
				_ => std::cmp::Ordering::Equal,
			};
			match sort_dir {
				SortDirection::Ascending => cmp,
				SortDirection::Descending => cmp.reverse(),
			}
		});

		if let Some(Filter::Pid(pid)) =
			filters.iter().find(|f| matches!(f, Filter::Pid(_)))
		{
			if let Some(pos) = result.iter().position(|p| p.pid == *pid) {
				if pos > 0 {
					let pinned = result.remove(pos);
					result.insert(0, pinned);
				}
			}
		}

		let gen = self.view_state.borrow().generation;
		self.view_state.borrow_mut().cached =
			Some((ts, gen, Rc::new(result)));
		self.view_state.borrow().cached.as_ref().unwrap().2.clone()
	}

	pub fn pid_filter_mode(&self) -> PidFilterMode {
		self.view_state.borrow().pid_filter_mode
	}

	pub fn pinned_pid(&self) -> Option<i32> {
		self.view_state
			.borrow()
			.filters
			.iter()
			.find_map(|f| match f {
				Filter::Pid(pid) => Some(*pid),
				_ => None,
			})
	}

	#[hotpath::measure]
	pub fn proc_matches(&self, proc: &ProcessInfo, filter: &Filter) -> bool {
		match filter {
			Filter::Gui => proc.is_gui,
			Filter::User => proc.is_owned_by_current_user && !proc.is_gui,
			Filter::System => {
				!proc.is_kthread
					&& !proc.is_owned_by_current_user
					&& proc.ppid != 1
			}
			Filter::Services => {
				let procs = self.snapshot_cell.borrow();
				let runsv = pids_of_runsv(&procs.processes);
				let supervise = pids_of_supervise_daemon(&procs.processes);
				is_service(proc, self.init_system, &runsv, &supervise)
			}
			Filter::Kernel => proc.is_kthread,
			Filter::Parent => proc.has_children,
			Filter::Vram => {
				let vs = self.view_state.borrow();
				if vs.resource_view_mode == ResourceViewMode::Cumulative {
					self.compute_aggregate_cumulative_map()
						.get(&proc.pid)
						.and_then(|c| c.vram)
						.is_some()
				} else {
					proc.vram_bytes.is_some()
				}
			}
			Filter::Electron => proc.is_electron,
			Filter::ProcessState(c) => proc.state == *c,
			Filter::Username(s) => proc.user == *s,
			Filter::Pid(pid) => match self.pid_filter_mode() {
				PidFilterMode::AllDescendants => {
					proc.pid == *pid || self.is_descendant_of(proc.pid, *pid)
				}
				PidFilterMode::DirectChildren => {
					proc.pid == *pid || proc.ppid == *pid
				}
			},
		}
	}

	pub fn descendant_pids_of(&self, pid: i32) -> Vec<i32> {
		let procs = &self.snapshot_cell.borrow().processes;
		procs
			.iter()
			.filter(|p| self.is_descendant_of(p.pid, pid))
			.map(|p| p.pid)
			.collect()
	}

	pub fn pid_to_ppid_map(&self) -> HashMap<i32, i32> {
		self.snapshot_cell
			.borrow()
			.processes
			.iter()
			.map(|p| (p.pid, p.ppid))
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::data::config::ProcessesConfig;
	use crate::data::platform::system::InitSystem;
	use std::cell::RefCell;
	use std::rc::Rc;

	fn make_snapshot(
		procs: Vec<ProcessInfo>,
	) -> Rc<RefCell<Rc<SystemSnapshot>>> {
		let snap = SystemSnapshot {
			processes: procs,
			cpu: CpuInfo {
				cores: vec![],
				overall_percent: 0.0,
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
			gpu_backend: GpuBackend::None,
		};
		Rc::new(RefCell::new(Rc::new(snap)))
	}

	fn make_proc(pid: i32, ppid: i32) -> ProcessInfo {
		ProcessInfo {
			pid,
			ppid,
			name: format!("proc-{pid}"),
			user: "root".into(),
			state: 'S',
			command: format!("/usr/bin/proc-{pid}"),
			cgroup: String::new(),
			cpu_percent: pid as f32,
			mem_percent: 0.0,
			mem_rss: pid as u64 * 1024,
			vram_bytes: None,
			disk_read_bytes_per_sec: pid as f64,
			disk_write_bytes_per_sec: pid as f64,
			is_gui: false,
			is_kthread: false,
			is_owned_by_current_user: false,
			is_electron: false,
			electron_app_name: None,
			icon_name: None,
			has_children: false,
		}
	}

	fn make_delegate(procs: Vec<ProcessInfo>) -> ProcessTableDelegate {
		ProcessTableDelegate {
			snapshot_cell: make_snapshot(procs),
			cum_cache: Rc::new(RefCell::new(None)),
			pid_index: Rc::new(RefCell::new(None)),
			descendant_counts: Rc::new(RefCell::new(None)),
			view_state: Rc::new(RefCell::new(ViewState {
				generation: 0,
				filters: vec![],
				filter_mode: FilterMode::And,
				pid_filter_mode: PidFilterMode::AllDescendants,
				search: String::new(),
				sort_col: 2,
				sort_dir: SortDirection::Ascending,
				resource_view_mode: ResourceViewMode::SelfOnly,
				cached: None,
			})),
			column_visibility: ProcessesConfig::default(),
			init_system: InitSystem::Unknown,
		}
	}

	// ── is_col_hidden ──────────────────────────────────────────

	#[test]
	fn is_col_hidden_col_0_never_hidden() {
		let d = make_delegate(vec![]);
		assert!(!d.is_col_hidden(0));
	}

	#[test]
	fn is_col_hidden_name_visible_by_default() {
		let d = make_delegate(vec![]);
		assert!(!d.is_col_hidden(1));
	}

	#[test]
	fn is_col_hidden_after_hiding_name() {
		let mut d = make_delegate(vec![]);
		if let Some(entry) = d
			.column_visibility
			.columns
			.iter_mut()
			.find(|e| e.column == SortColumn::Name)
		{
			entry.visible = false;
		}
		assert!(d.is_col_hidden(1));
	}

	#[test]
	fn is_col_hidden_out_of_range_returns_false() {
		let d = make_delegate(vec![]);
		assert!(!d.is_col_hidden(999));
	}

	// ── is_descendant_of ──────────────────────────────────────

	#[test]
	fn descendant_direct_child() {
		let d = make_delegate(vec![make_proc(1, 0), make_proc(10, 1)]);
		assert!(d.is_descendant_of(10, 1));
	}

	#[test]
	fn descendant_grandchild() {
		let d = make_delegate(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(100, 10),
		]);
		assert!(d.is_descendant_of(100, 1));
	}

	#[test]
	fn descendant_not_descendant() {
		let d = make_delegate(vec![make_proc(1, 0), make_proc(10, 2)]);
		assert!(!d.is_descendant_of(10, 1));
	}

	#[test]
	fn descendant_pid_eq_ancestor_returns_false() {
		let d = make_delegate(vec![make_proc(1, 0), make_proc(10, 1)]);
		assert!(!d.is_descendant_of(1, 1));
	}

	#[test]
	fn descendant_ppid_zero_terminates() {
		let d = make_delegate(vec![make_proc(1, 0)]);
		assert!(!d.is_descendant_of(1, 100));
	}

	#[test]
	fn descendant_unknown_pid_returns_false() {
		let d = make_delegate(vec![make_proc(1, 0)]);
		assert!(!d.is_descendant_of(999, 1));
	}

	// ── count_descendants_of ──────────────────────────────────

	#[test]
	fn count_descendants_parent_with_children() {
		let d = make_delegate(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(20, 1),
			make_proc(30, 1),
		]);
		assert_eq!(d.count_descendants_of(1), 3);
	}

	#[test]
	fn count_descendants_leaf_has_none() {
		let d = make_delegate(vec![make_proc(1, 0), make_proc(10, 1)]);
		assert_eq!(d.count_descendants_of(10), 0);
	}

	#[test]
	fn count_descendants_nested() {
		let d = make_delegate(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(20, 10),
			make_proc(30, 20),
		]);
		assert_eq!(d.count_descendants_of(1), 3);
		assert_eq!(d.count_descendants_of(10), 2);
	}

	// ── ancestor_chain_of ─────────────────────────────────────

	#[test]
	fn ancestor_chain_leaf_to_root() {
		let d = make_delegate(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(100, 10),
		]);
		let chain = d.ancestor_chain_of(100);
		let pids: Vec<i32> = chain.iter().map(|p| p.pid).collect();
		assert_eq!(pids, vec![1, 10, 100]);
	}

	#[test]
	fn ancestor_chain_root_is_itself() {
		let d = make_delegate(vec![make_proc(1, 0), make_proc(10, 1)]);
		let chain = d.ancestor_chain_of(1);
		let pids: Vec<i32> = chain.iter().map(|p| p.pid).collect();
		assert_eq!(pids, vec![1]);
	}

	#[test]
	fn ancestor_chain_unknown_pid_empty() {
		let d = make_delegate(vec![make_proc(1, 0)]);
		let chain = d.ancestor_chain_of(999);
		assert!(chain.is_empty());
	}

	// ── compute_aggregate_cumulative_map ──────────────────────

	#[test]
	fn cum_map_parent_child_sum() {
		let d = make_delegate(vec![make_proc(1, 0), make_proc(10, 1)]);
		let map = d.compute_aggregate_cumulative_map();
		assert_eq!(map[&1].cpu, 1.0 + 10.0);
		assert_eq!(map[&1].mem_rss, 1024 + 10240);
		assert_eq!(map[&10].cpu, 10.0);
	}

	#[test]
	fn cum_map_multi_level_tree() {
		let d = make_delegate(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(100, 10),
			make_proc(200, 10),
		]);
		let map = d.compute_aggregate_cumulative_map();
		assert_eq!(map[&1].cpu, 1.0 + 10.0 + 100.0 + 200.0);
		assert_eq!(map[&10].cpu, 10.0 + 100.0 + 200.0);
		assert_eq!(map[&100].cpu, 100.0);
	}

	#[test]
	fn cum_map_no_children_self_values() {
		let d = make_delegate(vec![
			make_proc(1, 0),
			make_proc(10, 0),
			make_proc(20, 0),
		]);
		let map = d.compute_aggregate_cumulative_map();
		assert_eq!(map[&1].cpu, 1.0);
		assert_eq!(map[&10].cpu, 10.0);
		assert_eq!(map[&20].cpu, 20.0);
	}

	#[test]
	fn cum_map_cache_hit_second_call() {
		let d = make_delegate(vec![make_proc(1, 0), make_proc(10, 1)]);
		let map1 = d.compute_aggregate_cumulative_map();
		let map2 = d.compute_aggregate_cumulative_map();
		assert_eq!(map1[&1].cpu, map2[&1].cpu);
	}

	#[test]
	fn cum_map_vram_parent_none_child_some() {
		let d = make_delegate(vec![make_proc(1, 0), make_proc(10, 1)]);
		let new_snap = {
			let borrowed = d.snapshot_cell.borrow();
			let mut clone: SystemSnapshot = (**borrowed).clone();
			clone.processes[1].vram_bytes = Some(4096);
			clone
		};
		*d.snapshot_cell.borrow_mut() = Rc::new(new_snap);
		let map = d.compute_aggregate_cumulative_map();
		assert_eq!(map[&1].vram, Some(4096));
	}

	#[test]
	fn cum_map_vram_child_none_parent_some() {
		let d = make_delegate(vec![make_proc(1, 0), make_proc(10, 1)]);
		let new_snap = {
			let borrowed = d.snapshot_cell.borrow();
			let mut clone: SystemSnapshot = (**borrowed).clone();
			clone.processes[0].vram_bytes = Some(4096);
			clone
		};
		*d.snapshot_cell.borrow_mut() = Rc::new(new_snap);
		let map = d.compute_aggregate_cumulative_map();
		assert_eq!(map[&1].vram, Some(4096));
	}

	#[test]
	fn cum_map_empty() {
		let d = make_delegate(vec![]);
		let map = d.compute_aggregate_cumulative_map();
		assert!(map.is_empty());
	}

	// ── pid_filter_mode ───────────────────────────────────────

	#[test]
	fn pid_filter_mode_all_descendants() {
		let d = make_delegate(vec![]);
		assert_eq!(d.pid_filter_mode(), PidFilterMode::AllDescendants);
	}

	#[test]
	fn pid_filter_mode_direct_children() {
		let d = make_delegate(vec![]);
		d.view_state.borrow_mut().pid_filter_mode =
			PidFilterMode::DirectChildren;
		assert_eq!(d.pid_filter_mode(), PidFilterMode::DirectChildren);
	}

	// ── pinned_pid ────────────────────────────────────────────

	#[test]
	fn pinned_pid_none_when_no_pid_filter() {
		let d = make_delegate(vec![]);
		assert_eq!(d.pinned_pid(), None);
	}

	#[test]
	fn pinned_pid_returns_some() {
		let d = make_delegate(vec![]);
		d.view_state.borrow_mut().filters = vec![Filter::Pid(42)];
		assert_eq!(d.pinned_pid(), Some(42));
	}

	#[test]
	fn pinned_pid_with_other_filters_returns_none() {
		let d = make_delegate(vec![]);
		d.view_state.borrow_mut().filters = vec![Filter::Gui, Filter::Kernel];
		assert_eq!(d.pinned_pid(), None);
	}

	// ── descendant_pids_of ────────────────────────────────────

	#[test]
	fn descendant_pids_of_empty() {
		let d = make_delegate(vec![make_proc(1, 0)]);
		assert_eq!(d.descendant_pids_of(1), Vec::<i32>::new());
	}

	#[test]
	fn descendant_pids_of_children() {
		let d = make_delegate(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(20, 1),
			make_proc(30, 2),
		]);
		let mut pids = d.descendant_pids_of(1);
		pids.sort();
		assert_eq!(pids, vec![10, 20]);
	}

	// ── pid_to_ppid_map ───────────────────────────────────────

	#[test]
	fn pid_to_ppid_map_entries() {
		let d = make_delegate(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(100, 10),
		]);
		let map = d.pid_to_ppid_map();
		assert_eq!(map[&1], 0);
		assert_eq!(map[&10], 1);
		assert_eq!(map[&100], 10);
	}

	#[test]
	fn pid_to_ppid_map_size() {
		let d = make_delegate(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(20, 1),
		]);
		assert_eq!(d.pid_to_ppid_map().len(), 3);
	}

	#[test]
	fn pid_to_ppid_map_empty() {
		let d = make_delegate(vec![]);
		assert!(d.pid_to_ppid_map().is_empty());
	}

	// ── proc_matches ────────────────────────────────────────────

	#[test]
	fn proc_matches_gui() {
		let d = make_delegate(vec![]);
		let mut p = make_proc(1, 0);
		assert!(!d.proc_matches(&p, &Filter::Gui));
		p.is_gui = true;
		assert!(d.proc_matches(&p, &Filter::Gui));
	}

	#[test]
	fn proc_matches_user() {
		let d = make_delegate(vec![]);
		let mut p = make_proc(1, 0);
		p.is_owned_by_current_user = true;
		assert!(d.proc_matches(&p, &Filter::User));
		p.is_gui = true;
		assert!(!d.proc_matches(&p, &Filter::User));
	}

	#[test]
	fn proc_matches_system() {
		let d = make_delegate(vec![]);
		let p = make_proc(2, 100);
		assert!(d.proc_matches(&p, &Filter::System));
	}

	#[test]
	fn proc_matches_system_excludes_kthread() {
		let d = make_delegate(vec![]);
		let mut p = make_proc(2, 100);
		p.is_kthread = true;
		assert!(!d.proc_matches(&p, &Filter::System));
	}

	#[test]
	fn proc_matches_kernel() {
		let d = make_delegate(vec![]);
		let mut p = make_proc(2, 0);
		p.is_kthread = true;
		assert!(d.proc_matches(&p, &Filter::Kernel));
	}

	#[test]
	fn proc_matches_parent() {
		let d = make_delegate(vec![]);
		let mut p = make_proc(1, 0);
		assert!(!d.proc_matches(&p, &Filter::Parent));
		p.has_children = true;
		assert!(d.proc_matches(&p, &Filter::Parent));
	}

	#[test]
	fn proc_matches_vram_self_only() {
		let d = make_delegate(vec![]);
		let mut p = make_proc(1, 0);
		assert!(!d.proc_matches(&p, &Filter::Vram));
		p.vram_bytes = Some(4096);
		assert!(d.proc_matches(&p, &Filter::Vram));
	}

	#[test]
	fn proc_matches_vram_cumulative() {
		let mut parent = make_proc(1, 0);
		parent.vram_bytes = Some(4096);
		let mut child = make_proc(10, 1);
		child.vram_bytes = Some(2048);
		let d = make_delegate(vec![parent, child.clone()]);
		d.view_state.borrow_mut().resource_view_mode =
			ResourceViewMode::Cumulative;
		assert!(d.proc_matches(&child, &Filter::Vram));
		assert!(d.proc_matches(&make_proc(1, 0), &Filter::Vram));
	}

	#[test]
	fn proc_matches_electron() {
		let d = make_delegate(vec![]);
		let mut p = make_proc(1, 0);
		assert!(!d.proc_matches(&p, &Filter::Electron));
		p.is_electron = true;
		assert!(d.proc_matches(&p, &Filter::Electron));
	}

	#[test]
	fn proc_matches_process_state() {
		let d = make_delegate(vec![]);
		let p = make_proc(1, 0);
		assert!(d.proc_matches(&p, &Filter::ProcessState('S')));
		assert!(!d.proc_matches(&p, &Filter::ProcessState('R')));
	}

	#[test]
	fn proc_matches_username() {
		let d = make_delegate(vec![]);
		let p = make_proc(1, 0);
		assert!(d.proc_matches(&p, &Filter::Username("root".into())));
		assert!(!d.proc_matches(&p, &Filter::Username("alice".into())));
	}

	#[test]
	fn proc_matches_pid_all_descendants_direct_child() {
		let d = make_delegate(vec![make_proc(1, 0), make_proc(10, 1)]);
		let child = make_proc(10, 1);
		assert!(d.proc_matches(&child, &Filter::Pid(1)));
	}

	#[test]
	fn proc_matches_pid_all_descendants_grandchild() {
		let d = make_delegate(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(100, 10),
		]);
		let grandchild = make_proc(100, 10);
		assert!(d.proc_matches(&grandchild, &Filter::Pid(1)));
	}

	#[test]
	fn proc_matches_pid_direct_children_direct_child() {
		let d = make_delegate(vec![make_proc(1, 0), make_proc(10, 1)]);
		d.view_state.borrow_mut().pid_filter_mode =
			PidFilterMode::DirectChildren;
		let child = make_proc(10, 1);
		assert!(d.proc_matches(&child, &Filter::Pid(1)));
	}

	#[test]
	fn proc_matches_pid_direct_children_grandchild() {
		let d = make_delegate(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(100, 10),
		]);
		d.view_state.borrow_mut().pid_filter_mode =
			PidFilterMode::DirectChildren;
		let grandchild = make_proc(100, 10);
		assert!(!d.proc_matches(&grandchild, &Filter::Pid(1)));
	}

	#[test]
	fn proc_matches_services_ppid_1() {
		let d = make_delegate(vec![]);
		let p = make_proc(10, 1);
		assert!(d.proc_matches(&p, &Filter::Services));
	}

	#[test]
	fn proc_matches_services_ppid_not_1() {
		let d = make_delegate(vec![]);
		let p = make_proc(20, 100);
		assert!(!d.proc_matches(&p, &Filter::Services));
	}

	fn set_view_state(
		d: &ProcessTableDelegate,
		f: impl FnOnce(&mut ViewState),
	) {
		f(&mut d.view_state.borrow_mut());
	}

	#[test]
	fn filtered_sorted_rows_empty() {
		let d = make_delegate(vec![]);
		let rows = d.filtered_sorted_rows();
		assert!(rows.is_empty());
	}

	#[test]
	fn filtered_sorted_rows_no_search_no_filters_sort_by_pid() {
		let procs =
			vec![make_proc(30, 1), make_proc(10, 1), make_proc(20, 1)];
		let d = make_delegate(procs);
		let rows = d.filtered_sorted_rows();
		assert_eq!(rows.len(), 3);
		assert_eq!(rows[0].pid, 10);
		assert_eq!(rows[1].pid, 20);
		assert_eq!(rows[2].pid, 30);
	}

	#[test]
	fn filtered_sorted_rows_sort_by_name() {
		let procs = vec![
			make_proc_named(1, "c-process"),
			make_proc_named(2, "a-process"),
			make_proc_named(3, "b-process"),
		];
		let d = make_delegate(procs);
		set_view_state(&d, |s| {
			s.sort_col = 1;
			s.sort_dir = SortDirection::Ascending;
		});
		let rows = d.filtered_sorted_rows();
		assert_eq!(rows[0].name, "a-process");
		assert_eq!(rows[1].name, "b-process");
		assert_eq!(rows[2].name, "c-process");
	}

	#[test]
	fn filtered_sorted_rows_sort_by_cpu_descending() {
		let procs = vec![
			make_proc_with_cpu(1, 10.0),
			make_proc_with_cpu(2, 50.0),
			make_proc_with_cpu(3, 30.0),
		];
		let d = make_delegate(procs);
		set_view_state(&d, |s| {
			s.sort_col = 5;
			s.sort_dir = SortDirection::Descending;
		});
		let rows = d.filtered_sorted_rows();
		assert_eq!(rows[0].cpu_percent, 50.0);
		assert_eq!(rows[1].cpu_percent, 30.0);
		assert_eq!(rows[2].cpu_percent, 10.0);
	}

	#[test]
	fn filtered_sorted_rows_search_by_name() {
		let procs = vec![
			make_proc_named(1, "firefox"),
			make_proc_named(2, "bash"),
			make_proc_named(3, "firewalld"),
		];
		let d = make_delegate(procs);
		set_view_state(&d, |s| {
			s.search = "fire".into();
		});
		let rows = d.filtered_sorted_rows();
		assert_eq!(rows.len(), 2);
		let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
		assert!(names.contains(&"firefox"));
		assert!(names.contains(&"firewalld"));
	}

	#[test]
	fn filtered_sorted_rows_search_by_pid_partial() {
		let procs = vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(100, 10),
			make_proc(99, 10),
		];
		let d = make_delegate(procs);
		set_view_state(&d, |s| {
			s.search = "10".into();
		});
		let rows = d.filtered_sorted_rows();
		assert!(!rows.is_empty());
		let pids: Vec<i32> = rows.iter().map(|r| r.pid).collect();
		assert!(pids.contains(&10));
	}

	#[test]
	fn filtered_sorted_rows_search_no_results() {
		let procs =
			vec![make_proc_named(1, "bash"), make_proc_named(2, "vim")];
		let d = make_delegate(procs);
		set_view_state(&d, |s| {
			s.search = "zzz_nonexistent".into();
		});
		let rows = d.filtered_sorted_rows();
		assert!(rows.is_empty());
	}

	#[test]
	fn filtered_sorted_rows_filter_user() {
		let procs = vec![
			make_proc_with_user(1, "root", true),
			make_proc_with_user(2, "kyza", false),
			make_proc_with_user(3, "root", true),
		];
		let d = make_delegate(procs);
		set_view_state(&d, |s| {
			s.filters = vec![Filter::User];
		});
		let rows = d.filtered_sorted_rows();
		assert_eq!(rows.len(), 2);
		for r in rows.iter() {
			assert!(!r.is_gui);
			assert!(r.is_owned_by_current_user);
		}
	}

	#[test]
	fn filtered_sorted_rows_filter_gui() {
		let procs = vec![
			{
				let mut p = make_proc(1, 0);
				p.is_gui = true;
				p
			},
			make_proc(2, 1),
		];
		let d = make_delegate(procs);
		set_view_state(&d, |s| {
			s.filters = vec![Filter::Gui];
		});
		let rows = d.filtered_sorted_rows();
		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].pid, 1);
	}

	#[test]
	fn filtered_sorted_rows_filter_mode_and() {
		let procs = vec![
			{
				let mut p = make_proc(1, 0);
				p.is_kthread = true;
				p.has_children = true;
				p
			},
			{
				let mut p = make_proc(2, 0);
				p.is_kthread = true;
				p
			},
		];
		let d = make_delegate(procs);
		set_view_state(&d, |s| {
			s.filters = vec![Filter::Kernel, Filter::Parent];
			s.filter_mode = FilterMode::And;
		});
		let rows = d.filtered_sorted_rows();
		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].pid, 1);
	}

	#[test]
	fn filtered_sorted_rows_filter_mode_or() {
		let procs = vec![
			{
				let mut p = make_proc(1, 0);
				p.is_gui = true;
				p
			},
			{
				let mut p = make_proc(2, 0);
				p.is_owned_by_current_user = true;
				p
			},
			make_proc(3, 0),
		];
		let d = make_delegate(procs);
		set_view_state(&d, |s| {
			s.filters = vec![Filter::Gui, Filter::User];
			s.filter_mode = FilterMode::Or;
		});
		let rows = d.filtered_sorted_rows();
		assert_eq!(rows.len(), 2);
	}

	#[test]
	fn filtered_sorted_rows_pid_filter_pins_to_top() {
		let procs = vec![
			make_proc(50, 1),
			make_proc(10, 1),
			make_proc(90, 100),
			make_proc(100, 1),
		];
		let d = make_delegate(procs);
		set_view_state(&d, |s| {
			s.filters = vec![Filter::Pid(100)];
			s.pid_filter_mode = PidFilterMode::AllDescendants;
		});
		let rows = d.filtered_sorted_rows();
		assert!(!rows.is_empty());
		assert_eq!(rows[0].pid, 100);
	}

	#[test]
	fn filtered_sorted_rows_cumulative_view_mode() {
		let parent = {
			let mut p = make_proc(1, 0);
			p.cpu_percent = 10.0;
			p.mem_rss = 1000;
			p.disk_read_bytes_per_sec = 5.0;
			p.disk_write_bytes_per_sec = 3.0;
			p.has_children = true;
			p
		};
		let child = {
			let mut p = make_proc(2, 1);
			p.cpu_percent = 20.0;
			p.mem_rss = 2000;
			p.disk_read_bytes_per_sec = 7.0;
			p.disk_write_bytes_per_sec = 4.0;
			p
		};
		let procs = vec![parent, child];
		let d = make_delegate(procs);
		set_view_state(&d, |s| {
			s.sort_col = 5;
			s.sort_dir = SortDirection::Descending;
			s.resource_view_mode = ResourceViewMode::Cumulative;
		});
		let rows = d.filtered_sorted_rows();
		assert_eq!(rows.len(), 2);
		assert_eq!(rows[0].pid, 1);
		assert_eq!(rows[1].pid, 2);
	}

	#[test]
	fn filtered_sorted_rows_cache_invalidation_on_generation() {
		let procs = vec![make_proc(1, 0), make_proc(2, 0)];
		let d = make_delegate(procs.clone());
		set_view_state(&d, |s| {
			s.sort_col = 2;
			s.sort_dir = SortDirection::Ascending;
		});
		let r1 = d.filtered_sorted_rows();
		assert_eq!(r1[0].pid, 1);
		d.view_state.borrow_mut().generation += 1;
		let r2 = d.filtered_sorted_rows();
		assert_eq!(r2[0].pid, 1);
	}

	fn make_proc_named(pid: i32, name: &str) -> ProcessInfo {
		let mut p = make_proc(pid, 0);
		p.name = name.into();
		p
	}

	fn make_proc_with_user(
		pid: i32,
		user: &str,
		is_owned: bool,
	) -> ProcessInfo {
		let mut p = make_proc(pid, 0);
		p.user = user.into();
		p.is_owned_by_current_user = is_owned;
		p
	}

	fn make_proc_with_cpu(pid: i32, cpu: f32) -> ProcessInfo {
		let mut p = make_proc(pid, 0);
		p.cpu_percent = cpu;
		p
	}
}
