use crate::config_store::ConfigStore;
use crate::fuzzy::{best_fuzzy_score, fuzzy_match};
use crate::model::*;
use crate::processes::graph::ProcessGraph;
use crate::service_manager::{
	is_service, pids_of_runsv, pids_of_supervise_daemon, InitSystem,
};
use crate::state::{CumulativeResources, ViewState};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// The process engine: the snapshot plus every derived value (filtered rows,
/// the pipeline match set, cumulative map, the process graph) behind one
/// shared handle.
///
/// All caches are private inner `RefCell`s and every method takes `&self`, so
/// callers can hold an immutable `RefCell` borrow of the engine while calling
/// any of its methods — no re-entrant borrow is possible. The snapshot can
/// only be replaced via `set_snapshot`, which resets every cache; view-state
/// changes bump `ViewState.generation`, which `rows()` checks lazily.
pub struct ProcessEngine {
	snapshot: RefCell<Rc<SystemSnapshot>>,
	pub config: ConfigStore,
	pub init_system: InitSystem,
	pub view_state: Rc<RefCell<ViewState>>,
	rows_cache:
		RefCell<Option<(u64, Rc<Vec<ProcessSnapshot>>, Rc<HashSet<i32>>)>>,
	cum_cache: RefCell<Option<HashMap<i32, CumulativeResources>>>,
	graph: RefCell<Option<Rc<ProcessGraph>>>,
}

impl ProcessEngine {
	pub fn new(
		snapshot: Rc<SystemSnapshot>,
		config: ConfigStore,
		init_system: InitSystem,
		view_state: Rc<RefCell<ViewState>>,
	) -> Self {
		Self {
			snapshot: RefCell::new(snapshot),
			config,
			init_system,
			view_state,
			rows_cache: RefCell::new(None),
			cum_cache: RefCell::new(None),
			graph: RefCell::new(None),
		}
	}

	pub fn set_snapshot(&self, snapshot: Rc<SystemSnapshot>) {
		self.snapshot.replace(snapshot);
		self.rows_cache.borrow_mut().take();
		self.cum_cache.borrow_mut().take();
		self.graph.borrow_mut().take();
	}

	pub fn snapshot(&self) -> Rc<SystemSnapshot> {
		self.snapshot.borrow().clone()
	}

	pub fn graph(&self) -> Rc<ProcessGraph> {
		if let Some(ref g) = *self.graph.borrow() {
			return g.clone();
		}
		let g = Rc::new(ProcessGraph::new(self.snapshot()));
		*self.graph.borrow_mut() = Some(g.clone());
		g
	}

	pub fn is_col_hidden(&self, col_ix: usize) -> bool {
		if col_ix == 0 {
			return false;
		}
		let cfg = self.config.get();
		SortColumn::from_col_index(col_ix)
			.map(|col| !cfg.processes.is_col_visible(col))
			.unwrap_or(false)
	}

	pub fn is_descendant_of(&self, child_pid: i32, ancestor: i32) -> bool {
		self.graph().is_descendant_of(child_pid, ancestor)
	}

	pub fn count_descendants_of(&self, pid: i32) -> usize {
		self.graph().subtree_count_of(pid)
	}

	pub fn ancestor_chain_of(&self, target_pid: i32) -> Vec<ProcessSnapshot> {
		self.graph().ancestor_chain_of(target_pid)
	}

	#[hotpath::measure]
	pub fn compute_cumulative_map(
		&self,
	) -> HashMap<i32, CumulativeResources> {
		if let Some(ref cached) = *self.cum_cache.borrow() {
			return cached.clone();
		}
		let snapshot = self.snapshot.borrow();
		let procs = &snapshot.processes;
		if procs.is_empty() {
			return HashMap::new();
		}

		let mut cum: HashMap<i32, CumulativeResources> =
			HashMap::with_capacity(procs.len());
		let mut pid_map: HashMap<i32, &ProcessSnapshot> =
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
					vram: p.vram,
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
				parent.vram.nvidia += child_vram.nvidia;
				parent.vram.amd += child_vram.amd;
				parent.disk_read += child_dr;
				parent.disk_write += child_dw;
			}
		}

		*self.cum_cache.borrow_mut() = Some(cum.clone());
		cum
	}

	pub fn cum(&self, pid: i32) -> Option<CumulativeResources> {
		self.cum_cache
			.borrow()
			.as_ref()
			.and_then(|m| m.get(&pid))
			.cloned()
	}

	#[hotpath::measure]
	pub fn rows(&self) -> Rc<Vec<ProcessSnapshot>> {
		self.compute_pipeline().0
	}

	/// The pipeline's match set: pids passing search + non-Pid filters,
	/// regardless of any Pid scope. The tree consumes this and intersects it
	/// with its own scope.
	#[hotpath::measure]
	pub fn match_set(&self) -> Rc<HashSet<i32>> {
		self.compute_pipeline().1
	}

	/// The tree's matched set: `match_set` restricted to the pins' full
	/// descendant scopes (the tree always shows all descendants, ignoring
	/// `pid_filter_mode`).
	pub fn tree_match_set(&self) -> Rc<HashSet<i32>> {
		let matched = self.match_set();
		let pid_filters = self.pid_filters();
		if pid_filters.is_empty() {
			return matched;
		}
		let graph = self.graph();
		let mut scope = HashSet::new();
		for &pf in &pid_filters {
			scope.insert(pf);
			scope.extend(graph.descendants_of(pf));
		}
		Rc::new(matched.intersection(&scope).copied().collect())
	}

	fn pid_filters(&self) -> Vec<i32> {
		self.view_state
			.borrow()
			.filters
			.iter()
			.filter_map(|f| match f {
				Filter::Pid(pid) => Some(*pid),
				_ => None,
			})
			.collect()
	}

	fn pid_scope(&self, pid_filters: &[i32]) -> HashSet<i32> {
		let graph = self.graph();
		let mut scope = HashSet::new();
		for &pf in pid_filters {
			scope.insert(pf);
			match self.pid_filter_mode() {
				PidFilterMode::AllDescendants => {
					scope.extend(graph.descendants_of(pf))
				}
				PidFilterMode::DirectChildren => {
					scope.extend(graph.children_of(pf).iter().copied())
				}
			}
		}
		scope
	}

	#[hotpath::measure]
	fn compute_pipeline(
		&self,
	) -> (Rc<Vec<ProcessSnapshot>>, Rc<HashSet<i32>>) {
		let gen = self.view_state.borrow().generation;
		if let Some((cached_gen, ref rows, ref matched)) =
			*self.rows_cache.borrow()
		{
			if cached_gen == gen {
				return (rows.clone(), matched.clone());
			}
		}

		let (
			search,
			filters,
			filter_mode,
			resource_view_mode,
			sort_col,
			sort_dir,
		) = {
			let vs = self.view_state.borrow();
			(
				vs.search.to_lowercase(),
				vs.filters.clone(),
				vs.filter_mode,
				vs.resource_view_mode,
				vs.sort_col,
				vs.sort_dir,
			)
		};

		if resource_view_mode == ResourceViewMode::Cumulative {
			self.compute_cumulative_map();
		}

		let all = self.snapshot.borrow().processes.clone();
		let pid_filters = self.pid_filters();
		let non_pid_filters: Vec<&Filter> = filters
			.iter()
			.filter(|f| !matches!(f, Filter::Pid(_)))
			.collect();

		let mut matcher = nucleo::Matcher::new(nucleo::Config::DEFAULT);

		let matched: HashSet<i32> = all
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
				if non_pid_filters.is_empty() {
					return true;
				}
				match filter_mode {
					FilterMode::And => non_pid_filters
						.iter()
						.all(|f| self.proc_matches(p, f)),
					FilterMode::Or => non_pid_filters
						.iter()
						.any(|f| self.proc_matches(p, f)),
				}
			})
			.map(|p| p.pid)
			.collect();

		let mut result: Vec<ProcessSnapshot> = if pid_filters.is_empty() {
			all.into_iter()
				.filter(|p| matched.contains(&p.pid))
				.collect()
		} else {
			let scope = self.pid_scope(&pid_filters);
			all.into_iter()
				.filter(|p| {
					matched.contains(&p.pid) && scope.contains(&p.pid)
				})
				.collect()
		};

		let cum_borrow = self.cum_cache.borrow();

		result.sort_by(|a, b| {
			if !search.is_empty() {
				let sa = best_fuzzy_score(&search, a, &mut matcher);
				let sb = best_fuzzy_score(&search, b, &mut matcher);
				match sb.cmp(&sa) {
					std::cmp::Ordering::Equal => {}
					other => return other,
				}
			}
			let cmp = match sort_col {
				SortColumn::Name => {
					a.name.to_lowercase().cmp(&b.name.to_lowercase())
				}
				SortColumn::Pid => a.pid.cmp(&b.pid),
				SortColumn::User => {
					a.user.to_lowercase().cmp(&b.user.to_lowercase())
				}
				SortColumn::State => a.state.cmp(&b.state),
				SortColumn::Cpu => {
					let va = cum_borrow
						.as_ref()
						.and_then(|m| m.get(&a.pid))
						.map_or(a.cpu_percent, |c| c.cpu);
					let vb = cum_borrow
						.as_ref()
						.and_then(|m| m.get(&b.pid))
						.map_or(b.cpu_percent, |c| c.cpu);
					va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
				}
				SortColumn::Memory => {
					let va = cum_borrow
						.as_ref()
						.and_then(|m| m.get(&a.pid))
						.map_or(a.mem_rss, |c| c.mem_rss);
					let vb = cum_borrow
						.as_ref()
						.and_then(|m| m.get(&b.pid))
						.map_or(b.mem_rss, |c| c.mem_rss);
					va.cmp(&vb)
				}
				SortColumn::Vram => {
					let va = cum_borrow
						.as_ref()
						.and_then(|m| m.get(&a.pid))
						.map_or(a.vram.total(), |c| c.vram.total());
					let vb = cum_borrow
						.as_ref()
						.and_then(|m| m.get(&b.pid))
						.map_or(b.vram.total(), |c| c.vram.total());
					va.cmp(&vb)
				}
				SortColumn::DiskRead => {
					let va = cum_borrow
						.as_ref()
						.and_then(|m| m.get(&a.pid))
						.map_or(a.disk_read_bytes_per_sec, |c| c.disk_read);
					let vb = cum_borrow
						.as_ref()
						.and_then(|m| m.get(&b.pid))
						.map_or(b.disk_read_bytes_per_sec, |c| c.disk_read);
					va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
				}
				SortColumn::DiskWrite => {
					let va = cum_borrow
						.as_ref()
						.and_then(|m| m.get(&a.pid))
						.map_or(a.disk_write_bytes_per_sec, |c| c.disk_write);
					let vb = cum_borrow
						.as_ref()
						.and_then(|m| m.get(&b.pid))
						.map_or(b.disk_write_bytes_per_sec, |c| c.disk_write);
					va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
				}
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

		let rows = Rc::new(result);
		let matched = Rc::new(matched);
		*self.rows_cache.borrow_mut() =
			Some((gen, rows.clone(), matched.clone()));
		(rows, matched)
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
	pub fn proc_matches(
		&self,
		proc: &ProcessSnapshot,
		filter: &Filter,
	) -> bool {
		match filter {
			Filter::Gui => proc.is_gui,
			Filter::User => proc.is_owned_by_current_user && !proc.is_gui,
			Filter::System => {
				!proc.is_kthread
					&& !proc.is_owned_by_current_user
					&& proc.ppid != 1
			}
			Filter::Services => {
				let snapshot = self.snapshot.borrow();
				let runsv = pids_of_runsv(&snapshot.processes);
				let supervise = pids_of_supervise_daemon(&snapshot.processes);
				is_service(proc, self.init_system, &runsv, &supervise)
			}
			Filter::Kernel => proc.is_kthread,
			Filter::Parent => proc.has_children,
			Filter::Vram => {
				let vs = self.view_state.borrow();
				if vs.resource_view_mode == ResourceViewMode::Cumulative {
					self.compute_cumulative_map()
						.get(&proc.pid)
						.map_or(false, |c| !c.vram.is_empty())
				} else {
					!proc.vram.is_empty()
				}
			}
			Filter::Nvidia => {
				let vs = self.view_state.borrow();
				if vs.resource_view_mode == ResourceViewMode::Cumulative {
					self.compute_cumulative_map()
						.get(&proc.pid)
						.map_or(false, |c| c.vram.nvidia > 0)
				} else {
					proc.vram.nvidia > 0
				}
			}
			Filter::Amd => {
				let vs = self.view_state.borrow();
				if vs.resource_view_mode == ResourceViewMode::Cumulative {
					self.compute_cumulative_map()
						.get(&proc.pid)
						.map_or(false, |c| c.vram.amd > 0)
				} else {
					proc.vram.amd > 0
				}
			}
			Filter::Electron => proc.is_electron,
			Filter::ProcessState(c) => proc.state == *c,
			Filter::Username(s) => proc.user == *s,
			Filter::Pid(_) => false,
		}
	}

	pub fn descendant_pids_of(&self, pid: i32) -> Vec<i32> {
		self.graph().descendants_of(pid)
	}

	pub fn pid_to_ppid_map(&self) -> HashMap<i32, i32> {
		self.snapshot
			.borrow()
			.processes
			.iter()
			.map(|p| (p.pid, p.ppid))
			.collect()
	}
}
