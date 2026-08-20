use crate::delegate::ProcessTableDelegate;
use gpui::*;
use gpui_component::tree::TreeItem;
use gpuitop_core::model::*;
use gpuitop_core::processes::graph::ProcessGraph;
use std::collections::{HashMap, HashSet};

pub struct TreeData {
	pub items: Vec<TreeItem>,
	pub process_lookup: HashMap<i32, ProcessSnapshot>,
	pub shown_descendant_counts: HashMap<i32, usize>,
	pub subtree_counts: HashMap<i32, usize>,
}

impl TreeData {
	#[hotpath::measure]
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
		let snapshot = self.snapshot();
		let processes = &snapshot.processes;
		let vs_ref = self.view_state();
		let vs = vs_ref.borrow();
		let pid_filters: Vec<i32> = vs
			.filters
			.iter()
			.filter_map(|f| match f {
				Filter::Pid(p) => Some(*p),
				_ => None,
			})
			.collect();
		let search_term = vs.search.to_lowercase();
		let has_search = !search_term.is_empty();
		drop(vs);

		let matched = self.tree_match_set();
		let graph = self.graph();
		let display = graph.with_ancestors(&matched);
		let expand_pids = graph.ancestors_to_expand(&matched);

		let mut pid_lookup: HashMap<i32, ProcessSnapshot> = HashMap::new();
		for proc in processes {
			if display.contains(&proc.pid) {
				pid_lookup.insert(proc.pid, proc.clone());
			}
		}

		let mut matcher = gpuitop_core::fuzzy::FuzzyMatcher::new();
		let fuzzy_scores: HashMap<i32, (u32, u32)> = if has_search {
			pid_lookup
				.iter()
				.map(|(pid, proc)| {
					(*pid, matcher.best_score(&search_term, proc))
				})
				.collect()
		} else {
			HashMap::new()
		};
		let use_fuzzy_sort = has_search;

		fn build_subtree(
			pids: &[i32],
			graph: &ProcessGraph,
			display: &HashSet<i32>,
			pid_lookup: &HashMap<i32, ProcessSnapshot>,
			matched: &HashSet<i32>,
			expand_pids: &HashSet<i32>,
			fuzzy_scores: &HashMap<i32, (u32, u32)>,
			use_fuzzy_sort: bool,
		) -> Vec<TreeItem> {
			let mut sorted_pids = pids.to_vec();
			if use_fuzzy_sort {
				sorted_pids.sort_by(|a, b| {
					let sa = fuzzy_scores.get(a).copied().unwrap_or((0, 0));
					let sb = fuzzy_scores.get(b).copied().unwrap_or((0, 0));
					sb.cmp(&sa)
				});
			} else {
				sorted_pids.sort_unstable();
			}
			sorted_pids
				.iter()
				.filter_map(|pid| {
					let proc = pid_lookup.get(pid)?;
					let child_pids: Vec<i32> = graph
						.children_of(*pid)
						.iter()
						.filter(|c| display.contains(c))
						.copied()
						.collect();
					let has_children = !child_pids.is_empty();
					let is_match = matched.contains(pid);
					let status = if is_match { "match" } else { "ancestor" };
					let id = format!("pid-{pid}:{status}");
					let label = proc.name.clone();

					let mut item = TreeItem::new(
						SharedString::from(id),
						SharedString::from(label),
					);

					if has_children {
						let subtree = build_subtree(
							&child_pids,
							graph,
							display,
							pid_lookup,
							matched,
							expand_pids,
							fuzzy_scores,
							use_fuzzy_sort,
						);
						item = item.children(subtree);
					}

					if expand_pids.contains(pid) {
						item = item.expanded(true);
					}

					Some(item)
				})
				.collect()
		}

		let top_level_pids: Vec<i32> = if !pid_filters.is_empty() {
			pid_filters
				.into_iter()
				.filter(|p| display.contains(p))
				.collect()
		} else {
			let mut pids: Vec<i32> = pid_lookup
				.keys()
				.copied()
				.filter(|pid| {
					let ppid =
						pid_lookup.get(pid).map(|p| p.ppid).unwrap_or(0);
					!display.contains(&ppid)
				})
				.collect();
			pids.sort_unstable();
			pids
		};

		let items = build_subtree(
			&top_level_pids,
			&graph,
			&display,
			&pid_lookup,
			&matched,
			&expand_pids,
			&fuzzy_scores,
			use_fuzzy_sort,
		);

		TreeData {
			items,
			process_lookup: pid_lookup,
			shown_descendant_counts: graph.display_subtree_counts(&display),
			subtree_counts: graph.subtree_counts().clone(),
		}
	}
}
