#[derive(Default, Clone)]
struct CumulativeResources {
	cpu: f32,
	mem_rss: u64,
	vram: Option<u64>,
	disk_read: f64,
	disk_write: f64,
}

use crate::config::Config;
use crate::model::*;
use bytesize::ByteSize;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	breadcrumb::{Breadcrumb, BreadcrumbItem},
	button::{Button, ButtonVariants},
	input::{Input, InputEvent, InputState},
	menu::{DropdownMenu, PopupMenu, PopupMenuItem},
	skeleton::Skeleton,
	table::{
		Column, ColumnFixed, ColumnSort, DataTable, TableDelegate,
		TableEvent, TableState,
	},
	tag::Tag,
	ActiveTheme, Disableable, Icon, IconName, Sizable,
};
use std::cell::RefCell;
use std::rc::Rc;

struct ViewState {
	generation: u64,
	filters: Vec<Filter>,
	filter_mode: FilterMode,
	pid_filter_mode: PidFilterMode,
	search: String,
	sort_col: usize,
	sort_dir: ColumnSort,
	resource_view_mode: ResourceViewMode,
	cached: Option<(std::time::Instant, u64, Rc<Vec<ProcessInfo>>)>,
}

impl ViewState {
	fn mutate(state: &Rc<RefCell<ViewState>>, f: impl FnOnce(&mut ViewState)) {
		let mut s = state.borrow_mut();
		f(&mut s);
		s.generation = s.generation.wrapping_add(1);
	}
}

	pub struct ProcessesTab {
	snapshot_cell: Rc<RefCell<Rc<SystemSnapshot>>>,
	cum_cache: Rc<RefCell<Option<std::collections::HashMap<i32, CumulativeResources>>>>,
	pid_index: Rc<RefCell<Option<std::collections::HashMap<i32, usize>>>>,
	view_state: Rc<RefCell<ViewState>>,
	table_state: Option<Entity<TableState<ProcessTableDelegate>>>,
	input_state: Option<Entity<InputState>>,
	clear_search_on_pin: bool,
	needs_clear_input: bool,
	needs_set_input: Option<String>,
	needs_focus_input: bool,
	_events: Option<Subscription>,
	_subscriptions: Vec<Subscription>,
	has_data: bool,
	is_picking: std::sync::Arc<std::sync::atomic::AtomicBool>,
	pick_result: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl ProcessesTab {
	pub fn new(
		config: Config,
		snapshot: Rc<SystemSnapshot>,
		_cx: &mut Context<Self>,
	) -> Self {
		Self {
			snapshot_cell: Rc::new(RefCell::new(snapshot)),
			cum_cache: Rc::new(RefCell::new(None)),
			pid_index: Rc::new(RefCell::new(None)),
			view_state: Rc::new(RefCell::new(ViewState {
				generation: 0,
				filters: Vec::new(),
				filter_mode: FilterMode::And,
				pid_filter_mode: config.pid_filter_mode,
				search: String::new(),
				sort_col: 5,
				sort_dir: ColumnSort::Descending,
				resource_view_mode: config.resource_view_mode,
				cached: None,
			})),
			table_state: None,
			input_state: None,
			clear_search_on_pin: config.clear_search_on_pin,
			needs_clear_input: false,
			needs_set_input: None,
			needs_focus_input: false,
			_events: None,
			_subscriptions: Vec::new(),
			has_data: false,
			is_picking: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
			pick_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
		}
	}

	pub fn set_snapshot(&mut self, snapshot: Rc<SystemSnapshot>) {
		*self.snapshot_cell.borrow_mut() = snapshot;
		*self.cum_cache.borrow_mut() = None;
		*self.pid_index.borrow_mut() = None;
	}

	fn toggle_filter(&mut self, filter: Filter, cx: &mut Context<Self>) {
		ViewState::mutate(&self.view_state, |s| {
			if let Some(pos) = s.filters.iter().position(|f| *f == filter) {
				s.filters.remove(pos);
			} else {
				s.filters.push(filter);
			}
		});
		cx.notify();
	}

	fn toggle_filter_mode(&mut self, cx: &mut Context<Self>) {
		ViewState::mutate(&self.view_state, |s| {
			s.filter_mode = match s.filter_mode {
				FilterMode::And => FilterMode::Or,
				FilterMode::Or => FilterMode::And,
			};
		});
		cx.notify();
	}

	fn toggle_pid_filter_mode(&mut self, cx: &mut Context<Self>) {
		ViewState::mutate(&self.view_state, |s| {
			s.pid_filter_mode = match s.pid_filter_mode {
				PidFilterMode::AllDescendants => PidFilterMode::DirectChildren,
				PidFilterMode::DirectChildren => PidFilterMode::AllDescendants,
			};
		});
		cx.notify();
	}

	pub fn focus_input(&mut self, _cx: &mut Context<Self>) {
		self.needs_focus_input = true;
	}

	fn toggle_resource_view_mode(&mut self, cx: &mut Context<Self>) {
		ViewState::mutate(&self.view_state, |s| {
			s.resource_view_mode = match s.resource_view_mode {
				ResourceViewMode::SelfOnly => ResourceViewMode::Cumulative,
				ResourceViewMode::Cumulative => ResourceViewMode::SelfOnly,
			};
		});
		cx.notify();
	}

	fn get_delegate(&self) -> ProcessTableDelegate {
		ProcessTableDelegate {
			snapshot_cell: self.snapshot_cell.clone(),
			cum_cache: self.cum_cache.clone(),
			pid_index: self.pid_index.clone(),
			view_state: self.view_state.clone(),
		}
	}
}

	struct ProcessTableDelegate {
	snapshot_cell: Rc<RefCell<Rc<SystemSnapshot>>>,
	cum_cache: Rc<RefCell<Option<std::collections::HashMap<i32, CumulativeResources>>>>,
	pid_index: Rc<RefCell<Option<std::collections::HashMap<i32, usize>>>>,
	view_state: Rc<RefCell<ViewState>>,
}

impl ProcessTableDelegate {
	fn is_descendant_of(&self, child_pid: i32, ancestor: i32) -> bool {
		let procs = &self.snapshot_cell.borrow().processes;
		if self.pid_index.borrow().is_none() {
			let mut map =
				std::collections::HashMap::with_capacity(procs.len());
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

	fn count_descendants_of(&self, pid: i32) -> usize {
		self.snapshot_cell
			.borrow()
			.processes
			.iter()
			.filter(|p| self.is_descendant_of(p.pid, pid))
			.count()
	}

	fn ancestor_chain_of(&self, target_pid: i32) -> Vec<ProcessInfo> {
		let procs = &self.snapshot_cell.borrow().processes;
		if self.pid_index.borrow().is_none() {
			let mut map =
				std::collections::HashMap::with_capacity(procs.len());
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
	fn compute_aggregate_cumulative_map(
		&self,
	) -> std::collections::HashMap<i32, CumulativeResources> {
		if let Some(ref cached) = *self.cum_cache.borrow() {
			return cached.clone();
		}
		let procs = &self.snapshot_cell.borrow().processes;
		if procs.is_empty() {
			return std::collections::HashMap::new();
		}

		let mut cum: std::collections::HashMap<i32, CumulativeResources> =
			std::collections::HashMap::with_capacity(procs.len());
		let mut pid_map: std::collections::HashMap<i32, &ProcessInfo> =
			std::collections::HashMap::with_capacity(procs.len());
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

		let mut depth: std::collections::HashMap<i32, usize> =
			std::collections::HashMap::with_capacity(procs.len());
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
	fn filtered_sorted_rows(&self) -> Rc<Vec<ProcessInfo>> {
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
					if !fuzzy_match(&search, &p.name.to_lowercase(), &mut matcher)
						&& !p.pid.to_string().contains(&search)
						&& !fuzzy_match(&search, &p.user.to_lowercase(), &mut matcher)
						&& !fuzzy_match(&search, &p.command.to_lowercase(), &mut matcher)
						&& !fuzzy_match(
							&search,
							&p.electron_app_name
								.as_deref()
								.unwrap_or_default()
								.to_lowercase(),
							&mut matcher,
						)
					{
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
		let dir = sort_dir;
		let use_cum =
			col >= 5 && resource_view_mode == ResourceViewMode::Cumulative;

		let cum_map: std::collections::HashMap<i32, CumulativeResources> =
			if use_cum {
				self.compute_aggregate_cumulative_map()
			} else {
				std::collections::HashMap::new()
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
			match dir {
				ColumnSort::Ascending => cmp,
				ColumnSort::Descending => cmp.reverse(),
				_ => cmp,
			}
		});

		// Pin PidFilter target to top
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

	fn pid_filter_mode(&self) -> PidFilterMode {
		self.view_state.borrow().pid_filter_mode
	}

	fn pinned_pid(&self) -> Option<i32> {
		self.view_state.borrow().filters.iter().find_map(|f| match f {
			Filter::Pid(pid) => Some(*pid),
			_ => None,
		})
	}

	fn proc_matches(&self, proc: &ProcessInfo, filter: &Filter) -> bool {
		match filter {
			Filter::Gui => proc.is_gui,
			Filter::User => proc.is_owned_by_current_user && !proc.is_gui,
			Filter::System => {
				!proc.is_kthread
					&& !proc.is_owned_by_current_user
					&& proc.ppid != 1
			}
			Filter::Systemd => proc.ppid == 1,
			Filter::Kernel => proc.is_kthread,
			Filter::Parent => proc.has_children,
			Filter::Vram => proc.vram_bytes.is_some(),
			Filter::Electron => proc.is_electron,
			Filter::ProcessState(c) => proc.state == *c,
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
}

fn fuzzy_match(needle: &str, haystack: &str, matcher: &mut nucleo::Matcher) -> bool {
	nucleo_fuzzy_score(needle, haystack, matcher).is_some()
}

fn nucleo_fuzzy_score(
	needle: &str,
	haystack: &str,
	matcher: &mut nucleo::Matcher,
) -> Option<u32> {
	let pattern = nucleo::pattern::Pattern::new(
		needle,
		nucleo::pattern::CaseMatching::Ignore,
		nucleo::pattern::Normalization::Smart,
		nucleo::pattern::AtomKind::Fuzzy,
	);
	let hs = nucleo::Utf32String::from(haystack);
	pattern.score(hs.slice(..), matcher)
}

fn best_fuzzy_score(
	needle: &str,
	p: &ProcessInfo,
	matcher: &mut nucleo::Matcher,
) -> u32 {
	let name_score =
		nucleo_fuzzy_score(needle, &p.name.to_lowercase(), matcher)
			.unwrap_or(0);
	let cmd_score =
		nucleo_fuzzy_score(needle, &p.command.to_lowercase(), matcher)
			.unwrap_or(0);
	let electron_score = nucleo_fuzzy_score(
		needle,
		&p.electron_app_name
			.as_deref()
			.unwrap_or_default()
			.to_lowercase(),
		matcher,
	)
	.unwrap_or(0);
	name_score.max(cmd_score).max(electron_score)
}

fn theme_dark_or_light(cx: &App) -> bool {
	cx.theme().is_dark()
}

fn state_info(state: char, dark: bool) -> (&'static str, Hsla) {
	match (state, dark) {
		('R', true) => ("Running", hsla(140.0 / 360.0, 0.55, 0.50, 1.0)),
		('R', false) => ("Running", hsla(140.0 / 360.0, 0.50, 0.35, 1.0)),
		('S', true) => ("Sleeping", hsla(0.0 / 360.0, 0.0, 0.55, 1.0)),
		('S', false) => ("Sleeping", hsla(0.0 / 360.0, 0.0, 0.42, 1.0)),
		('D', true) => ("Disk Sleep", hsla(0.0 / 360.0, 0.60, 0.55, 1.0)),
		('D', false) => ("Disk Sleep", hsla(0.0 / 360.0, 0.55, 0.40, 1.0)),
		('Z', true) => ("Zombie", hsla(0.0 / 360.0, 0.55, 0.45, 1.0)),
		('Z', false) => ("Zombie", hsla(0.0 / 360.0, 0.50, 0.32, 1.0)),
		('T', true) => ("Stopped", hsla(40.0 / 360.0, 0.70, 0.50, 1.0)),
		('T', false) => ("Stopped", hsla(40.0 / 360.0, 0.65, 0.37, 1.0)),
		('t', true) => ("Tracing", hsla(40.0 / 360.0, 0.65, 0.55, 1.0)),
		('t', false) => ("Tracing", hsla(40.0 / 360.0, 0.60, 0.42, 1.0)),
		('I', true) => ("Idle", hsla(200.0 / 360.0, 0.40, 0.55, 1.0)),
		('I', false) => ("Idle", hsla(200.0 / 360.0, 0.35, 0.42, 1.0)),
		('X', true) => ("Dead", hsla(0.0 / 360.0, 0.40, 0.40, 1.0)),
		('X', false) => ("Dead", hsla(0.0 / 360.0, 0.35, 0.28, 1.0)),
		_ => (
			"Unknown",
			hsla(0.0 / 360.0, 0.0, if dark { 0.50 } else { 0.40 }, 1.0),
		),
	}
}

fn tag_color(tag: TagType, dark: bool) -> Hsla {
	match (tag, dark) {
		(TagType::Gui, true) => hsla(215.0 / 360.0, 0.75, 0.60, 1.0),
		(TagType::Gui, false) => hsla(215.0 / 360.0, 0.65, 0.40, 1.0),
		(TagType::User, true) => hsla(190.0 / 360.0, 0.75, 0.55, 1.0),
		(TagType::User, false) => hsla(190.0 / 360.0, 0.65, 0.35, 1.0),
		(TagType::System, true) => hsla(40.0 / 360.0, 0.80, 0.55, 1.0),
		(TagType::System, false) => hsla(40.0 / 360.0, 0.75, 0.38, 1.0),
		(TagType::Systemd, true) => hsla(140.0 / 360.0, 0.55, 0.50, 1.0),
		(TagType::Systemd, false) => hsla(140.0 / 360.0, 0.50, 0.35, 1.0),
		(TagType::Kernel, true) => hsla(0.0 / 360.0, 0.0, 0.55, 1.0),
		(TagType::Kernel, false) => hsla(0.0 / 360.0, 0.0, 0.38, 1.0),
		(TagType::Vram, true) => hsla(270.0 / 360.0, 0.60, 0.60, 1.0),
		(TagType::Vram, false) => hsla(270.0 / 360.0, 0.55, 0.42, 1.0),
		(TagType::Parent, true) => hsla(0.0 / 360.0, 0.0, 0.60, 1.0),
		(TagType::Parent, false) => hsla(0.0 / 360.0, 0.0, 0.45, 1.0),
		(TagType::Electron, true) => hsla(170.0 / 360.0, 0.70, 0.55, 1.0),
		(TagType::Electron, false) => hsla(170.0 / 360.0, 0.60, 0.38, 1.0),
	}
}

#[derive(Clone, Copy)]
enum TagType {
	Gui,
	User,
	System,
	Systemd,
	Kernel,
	Vram,
	Parent,
	Electron,
}

fn tag_icons(proc: &ProcessInfo, cx: &App) -> Vec<(IconName, Hsla)> {
	let dark = theme_dark_or_light(cx);
	let mut tags = Vec::new();
	if proc.is_gui {
		tags.push((IconName::LayoutDashboard, tag_color(TagType::Gui, dark)));
	}
	if proc.is_owned_by_current_user && !proc.is_gui {
		tags.push((IconName::User, tag_color(TagType::User, dark)));
	}
	if !proc.is_kthread && !proc.is_owned_by_current_user && proc.ppid != 1 {
		tags.push((IconName::Settings2, tag_color(TagType::System, dark)));
	}
	if proc.ppid == 1 {
		tags.push((
			IconName::SquareTerminal,
			tag_color(TagType::Systemd, dark),
		));
	}
	if proc.is_kthread {
		tags.push((IconName::Cpu, tag_color(TagType::Kernel, dark)));
	}
	if proc.vram_bytes.is_some() {
		tags.push((IconName::Eye, tag_color(TagType::Vram, dark)));
	}
	if proc.is_electron {
		tags.push((IconName::Globe, tag_color(TagType::Electron, dark)));
	}
	tags
}

impl TableDelegate for ProcessTableDelegate {
	fn columns_count(&self, _: &App) -> usize {
		10
	}

	fn rows_count(&self, _: &App) -> usize {
		self.filtered_sorted_rows().len()
	}

	fn column(&self, col_ix: usize, _: &App) -> Column {
		let (key, name, width, sort) = match col_ix {
			0 => ("tags", " ", 64.0, None),
			1 => ("name", "Name", 260.0, Some(ColumnSort::Ascending)),
			2 => ("pid", "PID", 72.0, Some(ColumnSort::Ascending)),
			3 => ("user", "User", 80.0, Some(ColumnSort::Ascending)),
			4 => ("state", "State", 72.0, None),
			5 => ("cpu", "CPU%", 90.0, Some(ColumnSort::Descending)),
			6 => ("mem", "Mem", 88.0, Some(ColumnSort::Descending)),
			7 => ("vram", "VRAM", 88.0, Some(ColumnSort::Descending)),
			8 => ("dread", "Disk R", 96.0, Some(ColumnSort::Descending)),
			9 => ("dwrite", "Disk W", 96.0, Some(ColumnSort::Descending)),
			_ => ("", "", 100.0, None),
		};
		Column {
			key: key.into(),
			name: name.into(),
			width: px(width),
			sort,
			resizable: col_ix != 0,
			movable: col_ix != 0,
			fixed: if col_ix == 0 {
				Some(ColumnFixed::Left)
			} else {
				None
			},
			min_width: px(24.0),
			max_width: px(800.0),
			..Default::default()
		}
	}

	fn render_th(
		&mut self,
		col_ix: usize,
		_window: &mut Window,
		cx: &mut Context<TableState<Self>>,
	) -> impl IntoElement {
		let icon = match col_ix {
			1 => Some(IconName::File),
			2 => Some(IconName::Dash),
			3 => Some(IconName::User),
			4 => Some(IconName::CircleCheck),
			5 => Some(IconName::Cpu),
			6 => Some(IconName::MemoryStick),
			7 => Some(IconName::Eye),
			8 => Some(IconName::HardDrive),
			9 => Some(IconName::HardDrive),
			_ => None,
		};
		let name = self.column(col_ix, cx).name.clone();
		div()
			.flex()
			.flex_row()
			.items_center()
			.gap(px(4.0))
			.when(icon.is_some(), |el| {
				el.child(
					Icon::new(icon.unwrap())
						.size(px(12.0))
						.text_color(cx.theme().muted_foreground),
				)
			})
			.child(name)
	}

	fn render_td(
		&mut self,
		row_ix: usize,
		col_ix: usize,
		_: &mut Window,
		cx: &mut Context<TableState<Self>>,
	) -> impl IntoElement {
		let rows = self.filtered_sorted_rows();
		let Some(proc) = rows.get(row_ix) else {
			return div().into_any();
		};

		let cum = self
			.cum_cache
			.borrow()
			.as_ref()
			.and_then(|m| m.get(&proc.pid))
			.cloned();

		match col_ix {
			0 => {
				let tags = tag_icons(proc, cx);
				let is_pinned = self.pinned_pid() == Some(proc.pid);
				div()
					.flex()
					.flex_row()
					.gap(px(2.0))
					.items_center()
					.when(is_pinned, |el| {
						el.child(
							Icon::new(IconName::Star)
								.size(px(12.0))
								.text_color(cx.theme().primary),
						)
					})
					.children(tags.into_iter().map(|(icon, color)| {
						Icon::new(icon)
							.size(px(12.0))
							.text_color(color)
							.into_any_element()
					}))
					.into_any()
			}
			1 => {
				let has_children = proc.has_children;
				let descendant_count = self.count_descendants_of(proc.pid);
				let show_badge = has_children && descendant_count > 0;
				let display_name = proc
					.electron_app_name
					.as_deref()
					.unwrap_or(&proc.name)
					.to_string();
				if show_badge {
					div()
						.flex()
						.flex_row()
						.items_center()
						.text_sm()
						.text_color(cx.theme().foreground)
						.child(display_name.clone())
						.child(
							div()
								.text_size(px(10.0))
								.text_color(
									cx.theme().muted_foreground.opacity(0.6),
								)
								.ml(px(4.0))
								.child(format!("(+{descendant_count})")),
						)
						.into_any()
				} else {
					div()
						.text_sm()
						.text_color(cx.theme().foreground)
						.child(display_name.clone())
						.into_any()
				}
			}
			2 => div()
				.text_sm()
				.text_color(cx.theme().foreground)
				.child(proc.pid.to_string())
				.into_any(),
			3 => div()
				.text_sm()
				.text_color(cx.theme().muted_foreground)
				.child(proc.user.clone())
				.into_any(),
			4 => {
				let (state_label, c) =
					state_info(proc.state, theme_dark_or_light(cx));
				div().text_sm().text_color(c).child(state_label).into_any()
			}
			5 => {
				let cpu_val =
					cum.as_ref().map_or(proc.cpu_percent, |c| c.cpu);
				let c = if cpu_val > 50.0 {
					cx.theme().danger
				} else if cpu_val > 20.0 {
					cx.theme().warning
				} else {
					cx.theme().foreground
				};
				div()
					.text_sm()
					.text_color(c)
					.text_align(TextAlign::Right)
					.child(format!("{:.1}", cpu_val))
					.into_any()
			}
			6 => {
				let mem_val =
					cum.as_ref().map_or(proc.mem_rss, |c| c.mem_rss);
				let mem_pct = if let Some(ref c) = cum {
					let total_mem = self.snapshot_cell.borrow().memory.total;
					if total_mem > 0 {
						c.mem_rss as f32 / total_mem as f32 * 100.0
					} else {
						0.0
					}
				} else {
					proc.mem_percent
				};
				let c = if mem_pct > 30.0 {
					cx.theme().danger
				} else if mem_pct > 10.0 {
					cx.theme().warning
				} else {
					cx.theme().foreground
				};
				div()
					.text_sm()
					.text_color(c)
					.text_align(TextAlign::Right)
					.child(ByteSize::b(mem_val).to_string())
					.into_any()
			}
			7 => {
				let vram_val =
					cum.as_ref().and_then(|c| c.vram).or(proc.vram_bytes);
				div()
					.text_sm()
					.text_color(cx.theme().muted_foreground)
					.text_align(TextAlign::Right)
					.child(match vram_val {
						Some(b) => ByteSize::b(b).to_string(),
						None => "—".into(),
					})
					.into_any()
			}
			8 => {
				let dread_val = cum
					.as_ref()
					.map_or(proc.disk_read_bytes_per_sec, |c| c.disk_read);
				div()
					.text_sm()
					.text_color(cx.theme().muted_foreground)
					.text_align(TextAlign::Right)
					.child(if dread_val > 0.0 {
						format!("{}/s", ByteSize::b(dread_val as u64))
					} else {
						"0".into()
					})
					.into_any()
			}
			9 => {
				let dwrite_val = cum
					.as_ref()
					.map_or(proc.disk_write_bytes_per_sec, |c| c.disk_write);
				div()
					.text_sm()
					.text_color(cx.theme().muted_foreground)
					.text_align(TextAlign::Right)
					.child(if dwrite_val > 0.0 {
						format!("{}/s", ByteSize::b(dwrite_val as u64))
					} else {
						"0".into()
					})
					.into_any()
			}
			_ => div().into_any(),
		}
	}

	fn perform_sort(
		&mut self,
		col_ix: usize,
		sort: ColumnSort,
		_: &mut Window,
		cx: &mut Context<TableState<Self>>,
	) {
		ViewState::mutate(&self.view_state, |s| {
			s.sort_col = col_ix;
			s.sort_dir = sort;
		});
		cx.notify();
	}

	fn context_menu(
		&mut self,
		row_ix: usize,
		mut menu: PopupMenu,
		_window: &mut Window,
		_cx: &mut Context<TableState<Self>>,
	) -> PopupMenu {
		let rows = self.filtered_sorted_rows();
		if let Some(proc) = rows.get(row_ix) {
			for item in
				crate::widgets::process_context::build_process_menu(proc)
			{
				menu = menu.item(item);
			}
		}
		menu
	}

	fn render_tr(
		&mut self,
		row_ix: usize,
		_window: &mut Window,
		_cx: &mut Context<TableState<Self>>,
	) -> Stateful<Div> {
		div().id(("row", row_ix))
	}

	fn render_empty(
		&mut self,
		_: &mut Window,
		cx: &mut Context<TableState<Self>>,
	) -> impl IntoElement {
		div()
			.size_full()
			.flex()
			.items_center()
			.justify_center()
			.py(px(32.0))
			.text_sm()
			.text_color(cx.theme().muted_foreground)
			.child("No processes match the current filters")
			.into_any_element()
	}
}

fn filter_icon(filter: &Filter) -> Option<IconName> {
	match filter {
		Filter::Gui => Some(IconName::LayoutDashboard),
		Filter::User => Some(IconName::User),
		Filter::System => Some(IconName::Settings2),
		Filter::Systemd => Some(IconName::SquareTerminal),
		Filter::Kernel => Some(IconName::Cpu),
		Filter::Parent => Some(IconName::FolderOpen),
		Filter::Vram => Some(IconName::Eye),
		Filter::Electron => Some(IconName::Globe),
		Filter::ProcessState(_) => Some(IconName::Heart),
		Filter::Pid(_) => None,
	}
}

fn filter_color(filter: &Filter, cx: &App) -> Hsla {
	let dark = theme_dark_or_light(cx);
	match filter {
		Filter::Gui => tag_color(TagType::Gui, dark),
		Filter::User => tag_color(TagType::User, dark),
		Filter::System => tag_color(TagType::System, dark),
		Filter::Systemd => tag_color(TagType::Systemd, dark),
		Filter::Kernel => tag_color(TagType::Kernel, dark),
		Filter::Vram => tag_color(TagType::Vram, dark),
		Filter::Parent => tag_color(TagType::Parent, dark),
		Filter::Electron => tag_color(TagType::Electron, dark),
		Filter::ProcessState(_) => tag_color(TagType::Gui, dark),
		Filter::Pid(_) => tag_color(TagType::Systemd, dark),
	}
}

fn render_filter_chip(
	filter: &Filter,
	label: &str,
	is_active: bool,
	is_type: bool,
	cx: &mut Context<ProcessesTab>,
) -> AnyElement {
	if is_active && is_type {
		let f = filter.clone();
		let icon = filter_icon(filter);
		let icon_color = filter_color(filter, cx);
		let label_str = label.to_string();
		div()
			.id(ElementId::Name(format!("filt-{label}").into()))
			.cursor(CursorStyle::PointingHand)
			.on_click(cx.listener(move |this, _, _, cx| {
				this.toggle_filter(f.clone(), cx);
			}))
			.child(
				Tag::info().child(
					div()
						.flex()
						.flex_row()
						.items_center()
						.gap(px(4.0))
						.when(icon.is_some(), |el| {
							el.child(
								Icon::new(icon.unwrap())
									.size(px(12.0))
									.text_color(icon_color),
							)
						})
						.child(label_str),
				),
			)
			.into_any_element()
	} else if is_active {
		// PidFilter active: shows ✕ to remove
		let f = filter.clone();
		Tag::success()
			.child(
				div()
					.flex()
					.flex_row()
					.items_center()
					.gap(px(4.0))
					.child(label.to_string())
					.child(
						div()
							.id(ElementId::Name(
								format!("rm-filt-{label}").into(),
							))
							.cursor(CursorStyle::PointingHand)
							.on_click(cx.listener(move |this, _, _, cx| {
								ViewState::mutate(&this.view_state, |s| {
									s.filters.retain(|flt| flt != &f);
								});
								cx.notify();
							}))
							.child(Icon::new(IconName::Close).size(px(10.0))),
					),
			)
			.into_any_element()
	} else {
		let f = filter.clone();
		let icon = filter_icon(filter);
		let icon_color = filter_color(filter, cx);
		let label_str = label.to_string();
		div()
			.id(ElementId::Name(format!("filt-{label}").into()))
			.cursor(CursorStyle::PointingHand)
			.on_click(cx.listener(move |this, _, _, cx| {
				this.toggle_filter(f.clone(), cx);
			}))
			.child(
				Tag::new().child(
					div()
						.flex()
						.flex_row()
						.items_center()
						.gap(px(4.0))
						.when(icon.is_some(), |el| {
							el.child(
								Icon::new(icon.unwrap())
									.size(px(12.0))
									.text_color(icon_color),
							)
						})
						.child(label_str),
				),
			)
			.into_any_element()
	}
}

fn get_state_item(
	state: char,
	active: &[char],
	view_state: Rc<RefCell<ViewState>>,
) -> PopupMenuItem {
	let label = state_label(state).to_string();
	let checked = active.contains(&state);
	PopupMenuItem::Item {
		icon: None,
		label: label.into(),
		disabled: false,
		checked,
		is_link: false,
		action: None,
		handler: Some(std::rc::Rc::new(move |_, _, _| {
			ViewState::mutate(&view_state, |s| {
				if let Some(pos) = s
					.filters
					.iter()
					.position(|f| *f == Filter::ProcessState(state))
				{
					s.filters.remove(pos);
				} else {
					s.filters.push(Filter::ProcessState(state));
				}
			});
		})),
	}
}

impl Render for ProcessesTab {
	fn render(
		&mut self,
		window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		let snapshot = self.snapshot_cell.borrow().clone();
		self.has_data = !snapshot.processes.is_empty();
		let has_data = self.has_data;
		drop(snapshot);

		if let Some(app_id) = self.pick_result.lock().unwrap().take() {
			self.is_picking.store(false, std::sync::atomic::Ordering::Relaxed);
			let keyword = app_id.rsplit('.').next().unwrap_or(&app_id).to_string();
			self.needs_set_input = Some(keyword);
			cx.notify();
		}

		if self.table_state.is_none() {
			let delegate = self.get_delegate();
			let state = cx.new(|cx| TableState::new(delegate, window, cx));
			let events = cx.subscribe(
				&state,
				|this: &mut ProcessesTab, _state, event: &TableEvent, cx| {
					if let TableEvent::DoubleClickedRow(row_ix) = event {
						let pid = this
							.get_delegate()
							.filtered_sorted_rows()
							.get(*row_ix)
							.map(|p| p.pid);
						if let Some(pid) = pid {
							ViewState::mutate(&this.view_state, |s| {
								s.filters.clear();
								s.filters.push(Filter::Pid(pid));
							});
							if this.clear_search_on_pin {
								ViewState::mutate(&this.view_state, |s| {
									s.search.clear();
								});
								this.needs_clear_input = true;
							}
							cx.notify();
						}
					}
				},
			);
			self._events = Some(events);
			self.table_state = Some(state);
			cx.notify();
		}

		let table_state = self.table_state.as_ref().unwrap();

		if self.input_state.is_none() {
			let input_state = cx.new(|cx| {
				InputState::new(window, cx)
					.placeholder("Search by name, PID, user...")
			});
			let view_state = self.view_state.clone();
			let is_clone = input_state.clone();
			self._subscriptions = vec![cx.subscribe_in(
				&input_state,
				window,
				move |_, _, ev: &InputEvent, _, cx| match ev {
					InputEvent::Change => {
						let value = is_clone.read(cx).value();
						ViewState::mutate(&view_state, |s| {
							s.search = value.to_string();
						});
					}
					_ => {}
				},
			)];
			self.input_state = Some(input_state);
		}

		if self.needs_clear_input {
			if let Some(ref is) = self.input_state {
				is.update(cx, |state, cx| {
					state.set_value(String::new(), window, cx);
				});
			}
			self.needs_clear_input = false;
		}

		if let Some(value) = self.needs_set_input.take() {
			if let Some(ref is) = self.input_state {
				let vs = self.view_state.clone();
				is.update(cx, |state, cx| {
					state.set_value(value.clone(), window, cx);
				});
				ViewState::mutate(&vs, |s| {
					s.search = value;
				});
			}
		}

		if self.needs_focus_input {
			self.needs_focus_input = false;
			if let Some(ref is) = self.input_state {
				let is_clone = is.clone();
				cx.on_next_frame(window, move |_, w, cx| {
					is_clone.update(cx, |state, cx| {
						state.focus(w, cx);
					});
				});
			}
		}

		let snapshot = self.snapshot_cell.borrow().clone();
		let total_count = snapshot.processes.len();
		let filtered_count = self.get_delegate().filtered_sorted_rows().len();
		let active_filters = self.view_state.borrow().filters.len();
		let mode = self.view_state.borrow().filter_mode;

		let type_filters: Vec<Filter> = vec![
			Filter::Gui,
			Filter::User,
			Filter::System,
			Filter::Systemd,
			Filter::Kernel,
			Filter::Parent,
			Filter::Vram,
			Filter::Electron,
		];

		let pid_filters: Vec<(Filter, String)> = self
			.view_state
			.borrow()
			.filters
			.iter()
			.filter(|f| matches!(f, Filter::Pid(_)))
			.map(|f| {
				let label = f.label(&snapshot.processes);
				(f.clone(), label)
			})
			.collect();

		div()
			.size_full()
			.flex()
			.flex_col()
			.bg(cx.theme().background)
			.child(
				div()
					.w_full()
					.flex()
					.flex_col()
					.bg(cx.theme().background)
					.border_b_1()
					.border_color(cx.theme().border)
					.p(px(8.0))
					.gap(px(6.0))
					.child(
						div()
							.flex()
							.flex_row()
							.gap(px(8.0))
							.items_center()
                        .child(
                                Input::new(self.input_state.as_ref().unwrap())
                                    .small()
                                    .w_full()
                                    .prefix(
                                        Icon::new(IconName::Search)
                                            .size(px(12.0))
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                    .when(self.view_state.borrow().search.len() > 0, |input| {
										let view_state = self.view_state.clone();
										let input_state = self.input_state.as_ref().unwrap().clone();
										input.suffix(
											Button::new("clear-search")
												.ghost()
												.icon(Icon::new(IconName::Close).size(px(14.0)).text_color(cx.theme().muted_foreground))
												.small()
												.on_click(cx.listener(move |_, _, window, cx| {
													ViewState::mutate(&view_state, |s| {
														s.search.clear();
													});
													input_state.update(cx, |state, cx| {
														state.set_value(String::new(), window, cx);
													});
													cx.notify();
												})),
										)
									}),
                            )
							.child(
								Button::new("and-or")
									.ghost()
									.label(mode.to_string())
									.small()
									.on_click(cx.listener(
										|this, _, _, cx| {
											this.toggle_filter_mode(cx)
										},
									)),
							)
							.child({
								let view_label = self.view_state.borrow().resource_view_mode.to_string();
								Button::new("resource-view")
									.ghost()
									.label(view_label)
									.small()
									.on_click(cx.listener(
										|this, _, _, cx| {
											this.toggle_resource_view_mode(cx)
										},
									))
							})
							.child({
								let view_state = self.view_state.clone();
								Button::new("state-filter")
									.ghost()
									.label("State")
									.small()
									.dropdown_menu(move |menu, _window, _cx| {
										let vs = view_state.borrow();
										let active: Vec<char> = vs
											.filters
											.iter()
											.filter_map(|f| match f {
												Filter::ProcessState(c) => Some(*c),
												_ => None,
											})
											.collect();
										drop(vs);
										let vs = view_state.clone();
										menu.item(
											PopupMenuItem::Label("Process State".into()),
										)
										.item(
											get_state_item('R', &active, vs.clone()),
										)
										.item(
											get_state_item('S', &active, vs.clone()),
										)
										.item(
											get_state_item('D', &active, vs.clone()),
										)
										.item(
											get_state_item('Z', &active, vs.clone()),
										)
										.item(
											get_state_item('T', &active, vs.clone()),
										)
										.item(
											get_state_item('I', &active, vs.clone()),
										)
									})
							})
							.child({
								let pick_result = self.pick_result.clone();
								let is_picking = self.is_picking.clone();
								let picking = self.is_picking.load(std::sync::atomic::Ordering::Relaxed);
								Button::new("pick-window")
									.ghost()
									.label(if picking { "Picking…" } else { "Pick" })
									.small()
									.disabled(picking)
									.on_click(cx.listener(move |_this, _, _, cx| {
										is_picking.store(true, std::sync::atomic::Ordering::Relaxed);
										let r = pick_result.clone();
										let p = is_picking.clone();
										std::thread::spawn(move || {
											for _ in 0..50 {
												std::thread::sleep(std::time::Duration::from_millis(200));
												if let Some(id) = crate::wayland::get_focused_window_app_id() {
													*r.lock().unwrap() = Some(id);
													break;
												}
											}
											p.store(false, std::sync::atomic::Ordering::Relaxed);
										});
										cx.notify();
									}))
							}),
					)
					.child(
						div()
							.flex()
							.flex_row()
							.flex_wrap()
							.gap(px(4.0))
							.children(type_filters.iter().map(|f| {
								let is_active =
									self.view_state.borrow().filters.contains(f);
								render_filter_chip(
									f,
									&f.label(&[]),
									is_active,
									true,
									cx,
								)
							}))
							.children({
								let active_states: Vec<_> = self
									.view_state
									.borrow()
									.filters
									.iter()
									.filter(|f| matches!(f, Filter::ProcessState(_)))
									.cloned()
									.collect();
								let chips: Vec<AnyElement> = active_states.iter().map(|f| {
									render_filter_chip(
										f,
										&f.label(&[]),
										true,
										true,
										cx,
									)
								}).collect();
								chips
							}),
					)
                    .when(!pid_filters.is_empty(), |el| {
                        el.child(
                            div()
                                .flex().flex_row().flex_wrap().items_center().gap(px(4.0))
                                .child({
                                    div()
                                        .id(ElementId::Name("clear-pid-filter".into()))
                                        .cursor(CursorStyle::PointingHand)
                                        .flex().items_center().justify_center()
                                        .rounded(px(2.0))
                                        .hover(|s| s.bg(cx.theme().muted.opacity(0.15)))
                                        .child(Icon::new(IconName::Close).size(px(12.0)).text_color(cx.theme().muted_foreground))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            ViewState::mutate(&this.view_state, |s| {
                                                s.filters.retain(|f| !matches!(f, Filter::Pid(_)));
                                            });
                                            cx.notify();
                                        }))
                                })
                                .child({
                                    let mode_label = match self.view_state.borrow().pid_filter_mode {
                                        PidFilterMode::AllDescendants => "All",
                                        PidFilterMode::DirectChildren => "Direct",
                                    };
                                    div()
                                        .id(ElementId::Name("pid-filter-mode".into()))
                                        .cursor(CursorStyle::PointingHand)
                                        .px(px(6.0)).py(px(2.0)).rounded(px(3.0))
                                        .border_1().border_color(cx.theme().border)
                                        .text_size(px(10.0)).text_color(cx.theme().muted_foreground)
                                        .hover(|s| s.bg(cx.theme().muted.opacity(0.1)))
                                        .child(mode_label)
                                        .on_click(cx.listener(|this, _, _, cx| this.toggle_pid_filter_mode(cx)))
                                })
                                .children(pid_filters.iter().map(|(f, _label)| {
                                    if let Filter::Pid(pid) = f {
                                        let delegate = self.get_delegate();
                                        let chain = delegate.ancestor_chain_of(*pid);
                                        let child_count = delegate.count_descendants_of(*pid);
                                        let last_idx = chain.len().saturating_sub(1);
                                        let breadcrumb = Breadcrumb::new();
                                        let bc = chain.into_iter().enumerate().fold(breadcrumb, |bc, (i, proc)| {
                                            let is_last = i == last_idx;
                                            let name = proc.electron_app_name.as_deref().unwrap_or(&proc.name);
                                            let label = if is_last {
                                                if child_count > 0 {
                                                    format!("{} (+{})", name, child_count)
                                                } else {
                                                    name.to_string()
                                                }
                                            } else {
                                                name.to_string()
                                            };
                                            let pid = proc.pid;
                                            bc.child(
                                                BreadcrumbItem::new(label)
                                                    .disabled(is_last)
                                                    .on_click({
                                                        let view_state = self.view_state.clone();
                                                        move |_, _, _| {
                                                            ViewState::mutate(&view_state, |s| {
                                                                s.filters.clear();
                                                                s.filters.push(Filter::Pid(pid));
                                                            });
                                                        }
                                                    }),
                                            )
                                        });
                                        div().child(bc).into_any_element()
                                    } else {
                                        div().into_any_element()
                                    }
                                })),
                        )
                    }),
			)
			.child(div().flex_grow(1.0).size_full().child(
				if has_data {
					DataTable::new(table_state).stripe(true).bordered(false).into_any_element()
				} else {
					Skeleton::new().into_any_element()
				},
			))
			.child(
				div()
					.w_full()
					.h(px(24.0))
					.px(px(8.0))
					.bg(cx.theme().background)
					.border_t_1()
					.border_color(cx.theme().border)
					.flex()
					.flex_row()
					.items_center()
					.gap(px(16.0))
					.text_size(px(11.0))
					.text_color(cx.theme().muted_foreground)
					.child(format!("{filtered_count} of {total_count}"))
					.when(active_filters > 0, |el| {
						el.child(format!(
							"{active_filters} filter{} ({})",
							if active_filters > 1 { "s" } else { "" },
							mode
						))
					}),
			)
			.into_any_element()
	}
}
