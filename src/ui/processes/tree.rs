use crate::data::model::*;
use crate::data::processes::delegate::ProcessTableDelegate;
use gpui::*;
use gpui_component::tree::TreeItem;
use std::collections::{HashMap, HashSet};

pub struct TreeData {
	pub items: Vec<TreeItem>,
	pub process_lookup: HashMap<i32, ProcessSnapshot>,
	pub descendant_counts: HashMap<i32, usize>,
}

impl TreeData {
	pub fn preserve_expand_from(&mut self, old: &TreeData) {
		let old_state = Self::collect_expand_state(&old.items);
		self.items = Self::apply_expand_state(
			std::mem::take(&mut self.items),
			&old_state,
		);
	}

	fn collect_expand_state(items: &[TreeItem]) -> HashMap<String, bool> {
		let mut state = HashMap::new();
		for item in items {
			state.insert(item.id.to_string(), item.is_expanded());
			state.extend(Self::collect_expand_state(&item.children));
		}
		state
	}

	fn apply_expand_state(
		items: Vec<TreeItem>,
		old_state: &HashMap<String, bool>,
	) -> Vec<TreeItem> {
		items
			.into_iter()
			.map(|mut item| {
				let id = item.id.to_string();
				item.children = Self::apply_expand_state(
					std::mem::take(&mut item.children),
					old_state,
				);
				if let Some(&was_expanded) = old_state.get(&id) {
					item = item.expanded(was_expanded);
				}
				item
			})
			.collect()
	}
}

impl ProcessTableDelegate {
	#[hotpath::measure]
	pub fn build_tree(&self) -> TreeData {
		let snapshot = self.snapshot_cell.borrow();
		let processes = &snapshot.processes;
		let vs = self.view_state.borrow();

		let pid_filters: Vec<i32> = vs
			.filters
			.iter()
			.filter_map(|f| {
				if let Filter::Pid(p) = f {
					Some(*p)
				} else {
					None
				}
			})
			.collect();

		let scoped_pids: Option<HashSet<i32>> = if pid_filters.is_empty() {
			None
		} else {
			let mut set: HashSet<i32> = HashSet::new();
			for &pf in &pid_filters {
				set.insert(pf);
				set.extend(self.descendant_pids_of(pf));
			}
			Some(set)
		};

		let non_pid_filters: Vec<&Filter> = vs
			.filters
			.iter()
			.filter(|f| !matches!(f, Filter::Pid(_)))
			.collect();
		let search_term = vs.search.to_lowercase();
		let has_search = !search_term.is_empty();

		let mut matcher = nucleo::Matcher::new(nucleo::Config::DEFAULT);

		let mut proc_matches = |proc: &ProcessSnapshot| -> bool {
			if let Some(ref scope) = scoped_pids {
				if !scope.contains(&proc.pid) {
					return false;
				}
			}
			let filter_pass = if non_pid_filters.is_empty() {
				true
			} else {
				match vs.filter_mode {
					FilterMode::And => non_pid_filters
						.iter()
						.all(|f| self.proc_matches(proc, f)),
					FilterMode::Or => non_pid_filters
						.iter()
						.any(|f| self.proc_matches(proc, f)),
				}
			};
			if !filter_pass {
				return false;
			}
			if has_search {
				if !crate::data::fuzzy::fuzzy_match(
					&search_term,
					&proc.name.to_lowercase(),
					&mut matcher,
				) && !proc.pid.to_string().contains(&search_term)
					&& !crate::data::fuzzy::fuzzy_match(
						&search_term,
						&proc.command.to_lowercase(),
						&mut matcher,
					) && !crate::data::fuzzy::fuzzy_match(
					&search_term,
					&proc
						.electron_app_name
						.as_deref()
						.unwrap_or_default()
						.to_lowercase(),
					&mut matcher,
				) {
					return false;
				}
			}
			true
		};

		let pid_to_ppid = self.pid_to_ppid_map();

		let mut matched_pids: HashSet<i32> = HashSet::new();

		for proc in processes {
			if let Some(ref scope) = scoped_pids {
				if !scope.contains(&proc.pid) {
					continue;
				}
			}
			if !proc_matches(proc) {
				continue;
			}
			matched_pids.insert(proc.pid);
		}

		let display_pids = crate::data::processes::tree::with_ancestors(
			&matched_pids,
			&pid_to_ppid,
		);

		let mut pid_lookup: HashMap<i32, ProcessSnapshot> = HashMap::new();
		let mut children_by_ppid: HashMap<i32, Vec<&ProcessSnapshot>> =
			HashMap::new();

		for proc in processes {
			if !display_pids.contains(&proc.pid) {
				continue;
			}
			pid_lookup.insert(proc.pid, proc.clone());
			children_by_ppid.entry(proc.ppid).or_default().push(proc);
		}

		for children in children_by_ppid.values_mut() {
			children.sort_by_key(|p| p.pid);
		}

		let expand_pids = crate::data::processes::tree::ancestors_to_expand(
			&matched_pids,
			&pid_lookup,
		);

		let root_proc = if !pid_filters.is_empty() {
			pid_filters.first().and_then(|p| pid_lookup.get(p).cloned())
		} else {
			None
		};

		let fuzzy_scores: HashMap<i32, u32> = if has_search {
			pid_lookup
				.iter()
				.map(|(pid, proc)| {
					(
						*pid,
						crate::data::fuzzy::best_fuzzy_score(
							&search_term,
							proc,
							&mut matcher,
						),
					)
				})
				.collect()
		} else {
			HashMap::new()
		};

		let use_fuzzy_sort = has_search;

		fn build_subtree(
			pids: &[i32],
			children_by_ppid: &HashMap<i32, Vec<&ProcessSnapshot>>,
			pid_lookup: &HashMap<i32, ProcessSnapshot>,
			matched_pids: &HashSet<i32>,
			expand_pids: &HashSet<i32>,
			fuzzy_scores: &HashMap<i32, u32>,
			use_fuzzy_sort: bool,
		) -> Vec<TreeItem> {
			let mut sorted_pids = pids.to_vec();
			if use_fuzzy_sort {
				sorted_pids.sort_by(|a, b| {
					let sa = fuzzy_scores.get(a).copied().unwrap_or(0);
					let sb = fuzzy_scores.get(b).copied().unwrap_or(0);
					sb.cmp(&sa)
				});
			} else {
				sorted_pids.sort();
			}
			sorted_pids
				.iter()
				.filter_map(|pid| {
					let proc = pid_lookup.get(pid)?;
					let has_children = children_by_ppid
						.get(pid)
						.map_or(false, |c| !c.is_empty());
					let is_match = matched_pids.contains(pid);
					let status = if is_match { "match" } else { "ancestor" };
					let id = format!("pid-{pid}:{status}");
					let label = proc.name.clone();

					let mut item = TreeItem::new(
						SharedString::from(id),
						SharedString::from(label),
					);

					if has_children {
						let child_pids: Vec<i32> = children_by_ppid[pid]
							.iter()
							.map(|p| p.pid)
							.collect();
						let subtree = build_subtree(
							&child_pids,
							children_by_ppid,
							pid_lookup,
							matched_pids,
							expand_pids,
							fuzzy_scores,
							use_fuzzy_sort,
						);
						item = item.children(subtree);
					}

					let should_expand = expand_pids.contains(pid);
					if should_expand {
						item = item.expanded(true);
					}

					Some(item)
				})
				.collect()
		}

		let all_display_pids: HashSet<i32> = display_pids.clone();

		let top_level_pids: Vec<i32> = if let Some(ref root) = root_proc {
			vec![root.pid]
		} else {
			let mut pids: Vec<i32> = pid_lookup
				.keys()
				.copied()
				.filter(|pid| {
					let ppid =
						pid_lookup.get(pid).map(|p| p.ppid).unwrap_or(0);
					!all_display_pids.contains(&ppid)
				})
				.collect();
			pids.sort();
			pids
		};

		let items = build_subtree(
			&top_level_pids,
			&children_by_ppid,
			&pid_lookup,
			&matched_pids,
			&expand_pids,
			&fuzzy_scores,
			use_fuzzy_sort,
		);

		TreeData {
			items,
			process_lookup: pid_lookup,
			descendant_counts: count_descendants(&children_by_ppid),
		}
	}
}

#[hotpath::measure]
fn count_descendants(
	children_by_ppid: &HashMap<i32, Vec<&ProcessSnapshot>>,
) -> HashMap<i32, usize> {
	fn count_recursive(
		pid: i32,
		children_by_ppid: &HashMap<i32, Vec<&ProcessSnapshot>>,
		memo: &mut HashMap<i32, usize>,
	) -> usize {
		if let Some(&cached) = memo.get(&pid) {
			return cached;
		}
		let count = children_by_ppid
			.get(&pid)
			.map(|children| {
				children.len()
					+ children
						.iter()
						.map(|c| {
							count_recursive(c.pid, children_by_ppid, memo)
						})
						.sum::<usize>()
			})
			.unwrap_or(0);
		memo.insert(pid, count);
		count
	}

	let mut memo = HashMap::new();
	let all_pids: Vec<i32> = children_by_ppid.keys().copied().collect();
	for pid in &all_pids {
		count_recursive(*pid, children_by_ppid, &mut memo);
	}
	memo
}
