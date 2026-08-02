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

	pub fn count_descendants_of(&self, pid: i32) -> usize {
		self.snapshot_cell
			.borrow()
			.processes
			.iter()
			.filter(|p| self.is_descendant_of(p.pid, pid))
			.count()
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
