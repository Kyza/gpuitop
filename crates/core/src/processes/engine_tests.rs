#[cfg(test)]
mod tests {
	use crate::config::Config;
	use crate::config_store::ConfigStore;
	use crate::model::*;
	use crate::processes::engine::ProcessEngine;
	use crate::service_manager::InitSystem;
	use crate::state::ViewState;
	use std::cell::RefCell;
	use std::rc::Rc;

	fn make_snapshot(procs: Vec<ProcessSnapshot>) -> Rc<SystemSnapshot> {
		let snap = SystemSnapshot {
			processes: procs,
			cpu: CpuInfo {
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
		};
		Rc::new(snap)
	}

	fn make_proc(pid: i32, ppid: i32) -> ProcessSnapshot {
		ProcessSnapshot {
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
			vram: VramUsage::default(),
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

	fn make_engine(procs: Vec<ProcessSnapshot>) -> Rc<ProcessEngine> {
		let snapshot = make_snapshot(procs);
		let view_state = Rc::new(RefCell::new(ViewState {
			generation: 0,
			filters: vec![],
			filter_mode: FilterMode::And,
			pid_filter_mode: PidFilterMode::AllDescendants,
			search: String::new(),
			sort_col: SortColumn::Pid,
			sort_dir: SortDirection::Ascending,
			resource_view_mode: ResourceViewMode::SelfOnly,
		}));
		Rc::new(ProcessEngine::new(
			snapshot,
			ConfigStore::new(Config::default()),
			InitSystem::Unknown,
			view_state,
		))
	}

	// ── is_col_hidden ──────────────────────────────────────────

	#[test]
	fn is_col_hidden_col_0_never_hidden() {
		let d = make_engine(vec![]);
		assert!(!d.is_col_hidden(0));
	}

	#[test]
	fn is_col_hidden_name_visible_by_default() {
		let d = make_engine(vec![]);
		assert!(!d.is_col_hidden(1));
	}

	#[test]
	fn is_col_hidden_after_hiding_name() {
		let d = make_engine(vec![]);
		d.config.mutate(|cfg| {
			if let Some(entry) = cfg
				.processes
				.columns
				.iter_mut()
				.find(|e| e.column == SortColumn::Name)
			{
				entry.visible = false;
			}
		});
		assert!(d.is_col_hidden(1));
	}

	#[test]
	fn is_col_hidden_out_of_range_returns_false() {
		let d = make_engine(vec![]);
		assert!(!d.is_col_hidden(999));
	}

	// ── is_descendant_of ──────────────────────────────────────

	#[test]
	fn descendant_direct_child() {
		let d = make_engine(vec![make_proc(1, 0), make_proc(10, 1)]);
		assert!(d.is_descendant_of(10, 1));
	}

	#[test]
	fn descendant_grandchild() {
		let d = make_engine(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(100, 10),
		]);
		assert!(d.is_descendant_of(100, 1));
	}

	#[test]
	fn descendant_not_descendant() {
		let d = make_engine(vec![make_proc(1, 0), make_proc(10, 2)]);
		assert!(!d.is_descendant_of(10, 1));
	}

	#[test]
	fn descendant_pid_eq_ancestor_returns_false() {
		let d = make_engine(vec![make_proc(1, 0), make_proc(10, 1)]);
		assert!(!d.is_descendant_of(1, 1));
	}

	#[test]
	fn descendant_ppid_zero_terminates() {
		let d = make_engine(vec![make_proc(1, 0)]);
		assert!(!d.is_descendant_of(1, 100));
	}

	#[test]
	fn descendant_unknown_pid_returns_false() {
		let d = make_engine(vec![make_proc(1, 0)]);
		assert!(!d.is_descendant_of(999, 1));
	}

	// ── count_descendants_of ──────────────────────────────────

	#[test]
	fn count_descendants_parent_with_children() {
		let d = make_engine(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(20, 1),
			make_proc(30, 1),
		]);
		assert_eq!(d.count_descendants_of(1), 3);
	}

	#[test]
	fn count_descendants_leaf_has_none() {
		let d = make_engine(vec![make_proc(1, 0), make_proc(10, 1)]);
		assert_eq!(d.count_descendants_of(10), 0);
	}

	#[test]
	fn count_descendants_nested() {
		let d = make_engine(vec![
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
		let d = make_engine(vec![
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
		let d = make_engine(vec![make_proc(1, 0), make_proc(10, 1)]);
		let chain = d.ancestor_chain_of(1);
		let pids: Vec<i32> = chain.iter().map(|p| p.pid).collect();
		assert_eq!(pids, vec![1]);
	}

	#[test]
	fn ancestor_chain_unknown_pid_empty() {
		let d = make_engine(vec![make_proc(1, 0)]);
		let chain = d.ancestor_chain_of(999);
		assert!(chain.is_empty());
	}

	// ── cumulative_map ────────────────────────────────────────

	#[test]
	fn cum_map_parent_child_sum() {
		let d = make_engine(vec![make_proc(1, 0), make_proc(10, 1)]);
		let map = d.compute_cumulative_map();
		assert_eq!(map[&1].cpu, 1.0 + 10.0);
		assert_eq!(map[&1].mem_rss, 1024 + 10240);
		assert_eq!(map[&10].cpu, 10.0);
	}

	#[test]
	fn cum_map_multi_level_tree() {
		let d = make_engine(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(100, 10),
			make_proc(200, 10),
		]);
		let map = d.compute_cumulative_map();
		assert_eq!(map[&1].cpu, 1.0 + 10.0 + 100.0 + 200.0);
		assert_eq!(map[&10].cpu, 10.0 + 100.0 + 200.0);
		assert_eq!(map[&100].cpu, 100.0);
	}

	#[test]
	fn cum_map_no_children_self_values() {
		let d = make_engine(vec![
			make_proc(1, 0),
			make_proc(10, 0),
			make_proc(20, 0),
		]);
		let map = d.compute_cumulative_map();
		assert_eq!(map[&1].cpu, 1.0);
		assert_eq!(map[&10].cpu, 10.0);
		assert_eq!(map[&20].cpu, 20.0);
	}

	#[test]
	fn cum_map_cache_hit_second_call() {
		let d = make_engine(vec![make_proc(1, 0), make_proc(10, 1)]);
		let map1 = d.compute_cumulative_map();
		let map2 = d.compute_cumulative_map();
		assert_eq!(map1[&1].cpu, map2[&1].cpu);
	}

	#[test]
	fn cum_map_vram_parent_none_child_some() {
		let d = make_engine(vec![make_proc(1, 0), make_proc(10, 1)]);
		let new_snap = {
			let mut clone = d.snapshot().as_ref().clone();
			clone.processes[1].vram.nvidia = 4096;
			clone
		};
		d.set_snapshot(Rc::new(new_snap));
		let map = d.compute_cumulative_map();
		assert_eq!(map[&1].vram.nvidia, 4096);
	}

	#[test]
	fn cum_map_vram_child_none_parent_some() {
		let d = make_engine(vec![make_proc(1, 0), make_proc(10, 1)]);
		let new_snap = {
			let mut clone = d.snapshot().as_ref().clone();
			clone.processes[0].vram.nvidia = 4096;
			clone
		};
		d.set_snapshot(Rc::new(new_snap));
		let map = d.compute_cumulative_map();
		assert_eq!(map[&1].vram.nvidia, 4096);
	}

	#[test]
	fn cum_map_vram_mixed_vendors_sum() {
		let d = make_engine(vec![make_proc(1, 0), make_proc(10, 1)]);
		let new_snap = {
			let mut clone = d.snapshot().as_ref().clone();
			clone.processes[0].vram.nvidia = 2048;
			clone.processes[1].vram.amd = 1024;
			clone
		};
		d.set_snapshot(Rc::new(new_snap));
		let map = d.compute_cumulative_map();
		assert_eq!(map[&1].vram.nvidia, 2048);
		assert_eq!(map[&1].vram.amd, 1024);
		assert_eq!(map[&1].vram.total(), 3072);
	}

	#[test]
	fn cum_map_empty() {
		let d = make_engine(vec![]);
		let map = d.compute_cumulative_map();
		assert!(map.is_empty());
	}

	// ── mid-frame snapshot swap ───────────────────────────────

	#[test]
	fn rows_reflect_snapshot_swapped_mid_frame() {
		let d = make_engine(vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(100, 10),
		]);
		assert_eq!(d.rows().len(), 3);
		assert_eq!(d.count_descendants_of(1), 2);

		let new_snap = {
			let mut clone = d.snapshot().as_ref().clone();
			clone.processes.truncate(1);
			clone
		};
		d.set_snapshot(Rc::new(new_snap));

		assert_eq!(d.rows().len(), 1);
		assert_eq!(d.rows()[0].pid, 1);
		assert_eq!(d.count_descendants_of(1), 0);
		assert_eq!(d.compute_cumulative_map().len(), 1);
	}

	// ── pid_filter_mode ───────────────────────────────────────

	#[test]
	fn pid_filter_mode_all_descendants() {
		let d = make_engine(vec![]);
		assert_eq!(d.pid_filter_mode(), PidFilterMode::AllDescendants);
	}

	#[test]
	fn pid_filter_mode_direct_children() {
		let d = make_engine(vec![]);
		d.view_state.borrow_mut().pid_filter_mode =
			PidFilterMode::DirectChildren;
		assert_eq!(d.pid_filter_mode(), PidFilterMode::DirectChildren);
	}

	// ── pinned_pid ────────────────────────────────────────────

	#[test]
	fn pinned_pid_none_when_no_pid_filter() {
		let d = make_engine(vec![]);
		assert_eq!(d.pinned_pid(), None);
	}

	#[test]
	fn pinned_pid_returns_some() {
		let d = make_engine(vec![]);
		d.view_state.borrow_mut().filters = vec![Filter::Pid(42)];
		assert_eq!(d.pinned_pid(), Some(42));
	}

	#[test]
	fn pinned_pid_with_other_filters_returns_none() {
		let d = make_engine(vec![]);
		d.view_state.borrow_mut().filters = vec![Filter::Gui, Filter::Kernel];
		assert_eq!(d.pinned_pid(), None);
	}

	// ── proc_matches ────────────────────────────────────────────

	#[test]
	fn proc_matches_gui() {
		let d = make_engine(vec![]);
		let mut p = make_proc(1, 0);
		assert!(!d.proc_matches(&p, &Filter::Gui));
		p.is_gui = true;
		assert!(d.proc_matches(&p, &Filter::Gui));
	}

	#[test]
	fn proc_matches_user() {
		let d = make_engine(vec![]);
		let mut p = make_proc(1, 0);
		p.is_owned_by_current_user = true;
		assert!(d.proc_matches(&p, &Filter::User));
		p.is_gui = true;
		assert!(!d.proc_matches(&p, &Filter::User));
	}

	#[test]
	fn proc_matches_system() {
		let d = make_engine(vec![]);
		let p = make_proc(2, 100);
		assert!(d.proc_matches(&p, &Filter::System));
	}

	#[test]
	fn proc_matches_system_excludes_kthread() {
		let d = make_engine(vec![]);
		let mut p = make_proc(2, 100);
		p.is_kthread = true;
		assert!(!d.proc_matches(&p, &Filter::System));
	}

	#[test]
	fn proc_matches_kernel() {
		let d = make_engine(vec![]);
		let mut p = make_proc(2, 0);
		p.is_kthread = true;
		assert!(d.proc_matches(&p, &Filter::Kernel));
	}

	#[test]
	fn proc_matches_parent() {
		let d = make_engine(vec![]);
		let mut p = make_proc(1, 0);
		assert!(!d.proc_matches(&p, &Filter::Parent));
		p.has_children = true;
		assert!(d.proc_matches(&p, &Filter::Parent));
	}

	#[test]
	fn proc_matches_vram_self_only() {
		let d = make_engine(vec![]);
		let mut p = make_proc(1, 0);
		assert!(!d.proc_matches(&p, &Filter::Vram));
		p.vram.nvidia = 4096;
		assert!(d.proc_matches(&p, &Filter::Vram));
	}

	#[test]
	fn proc_matches_vram_cumulative() {
		let mut parent = make_proc(1, 0);
		parent.vram.nvidia = 4096;
		let mut child = make_proc(10, 1);
		child.vram.nvidia = 2048;
		let d = make_engine(vec![parent, child.clone()]);
		d.view_state.borrow_mut().resource_view_mode =
			ResourceViewMode::Cumulative;
		assert!(d.proc_matches(&child, &Filter::Vram));
		assert!(d.proc_matches(&make_proc(1, 0), &Filter::Vram));
	}

	#[test]
	fn proc_matches_nvidia_and_amd() {
		let mut nvidia = make_proc(1, 0);
		nvidia.vram.nvidia = 1024;
		let mut amd = make_proc(2, 0);
		amd.vram.amd = 1024;
		let d = make_engine(vec![nvidia.clone(), amd.clone()]);
		assert!(d.proc_matches(&nvidia, &Filter::Nvidia));
		assert!(!d.proc_matches(&nvidia, &Filter::Amd));
		assert!(d.proc_matches(&amd, &Filter::Amd));
		assert!(!d.proc_matches(&amd, &Filter::Nvidia));
		assert!(d.proc_matches(&nvidia, &Filter::Vram));
	}

	#[test]
	fn proc_matches_electron() {
		let d = make_engine(vec![]);
		let mut p = make_proc(1, 0);
		assert!(!d.proc_matches(&p, &Filter::Electron));
		p.is_electron = true;
		assert!(d.proc_matches(&p, &Filter::Electron));
	}

	#[test]
	fn proc_matches_process_state() {
		let d = make_engine(vec![]);
		let p = make_proc(1, 0);
		assert!(d.proc_matches(&p, &Filter::ProcessState('S')));
		assert!(!d.proc_matches(&p, &Filter::ProcessState('R')));
	}

	#[test]
	fn proc_matches_username() {
		let d = make_engine(vec![]);
		let p = make_proc(1, 0);
		assert!(d.proc_matches(&p, &Filter::Username("root".into())));
		assert!(!d.proc_matches(&p, &Filter::Username("alice".into())));
	}

	#[test]
	fn proc_matches_services_ppid_1() {
		let d = make_engine(vec![]);
		let p = make_proc(10, 1);
		assert!(d.proc_matches(&p, &Filter::Services));
	}

	#[test]
	fn proc_matches_services_ppid_not_1() {
		let d = make_engine(vec![]);
		let p = make_proc(20, 100);
		assert!(!d.proc_matches(&p, &Filter::Services));
	}

	fn set_view_state(d: &Rc<ProcessEngine>, f: impl FnOnce(&mut ViewState)) {
		f(&mut d.view_state.borrow_mut());
	}

	#[test]
	fn rows_empty() {
		let d = make_engine(vec![]);
		let rows = d.rows();
		assert!(rows.is_empty());
	}

	#[test]
	fn rows_no_search_no_filters_sort_by_pid() {
		let procs =
			vec![make_proc(30, 1), make_proc(10, 1), make_proc(20, 1)];
		let d = make_engine(procs);
		let rows = d.rows();
		assert_eq!(rows.len(), 3);
		assert_eq!(rows[0].pid, 10);
		assert_eq!(rows[1].pid, 20);
		assert_eq!(rows[2].pid, 30);
	}

	#[test]
	fn rows_sort_by_name() {
		let procs = vec![
			make_proc_named(1, "c-process"),
			make_proc_named(2, "a-process"),
			make_proc_named(3, "b-process"),
		];
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.sort_col = SortColumn::Name;
			s.sort_dir = SortDirection::Ascending;
		});
		let rows = d.rows();
		assert_eq!(rows[0].name, "a-process");
		assert_eq!(rows[1].name, "b-process");
		assert_eq!(rows[2].name, "c-process");
	}

	#[test]
	fn rows_sort_by_cpu_descending() {
		let procs = vec![
			make_proc_with_cpu(1, 10.0),
			make_proc_with_cpu(2, 50.0),
			make_proc_with_cpu(3, 30.0),
		];
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.sort_col = SortColumn::Cpu;
			s.sort_dir = SortDirection::Descending;
		});
		let rows = d.rows();
		assert_eq!(rows[0].cpu_percent, 50.0);
		assert_eq!(rows[1].cpu_percent, 30.0);
		assert_eq!(rows[2].cpu_percent, 10.0);
	}

	#[test]
	fn rows_search_by_name() {
		let procs = vec![
			make_proc_named(1, "firefox"),
			make_proc_named(2, "bash"),
			make_proc_named(3, "firewalld"),
		];
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.search = "fire".into();
		});
		let rows = d.rows();
		assert_eq!(rows.len(), 2);
		let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
		assert!(names.contains(&"firefox"));
		assert!(names.contains(&"firewalld"));
	}

	#[test]
	fn rows_search_by_pid_partial() {
		let procs = vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(100, 10),
			make_proc(99, 10),
		];
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.search = "10".into();
		});
		let rows = d.rows();
		assert!(!rows.is_empty());
		let pids: Vec<i32> = rows.iter().map(|r| r.pid).collect();
		assert!(pids.contains(&10));
	}

	#[test]
	fn rows_search_no_results() {
		let procs =
			vec![make_proc_named(1, "bash"), make_proc_named(2, "vim")];
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.search = "zzz_nonexistent".into();
		});
		let rows = d.rows();
		assert!(rows.is_empty());
	}

	#[test]
	fn rows_filter_user() {
		let procs = vec![
			make_proc_with_user(1, "root", true),
			make_proc_with_user(2, "kyza", false),
			make_proc_with_user(3, "root", true),
		];
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.filters = vec![Filter::User];
		});
		let rows = d.rows();
		assert_eq!(rows.len(), 2);
		for r in rows.iter() {
			assert!(!r.is_gui);
			assert!(r.is_owned_by_current_user);
		}
	}

	#[test]
	fn rows_filter_gui() {
		let procs = vec![
			{
				let mut p = make_proc(1, 0);
				p.is_gui = true;
				p
			},
			make_proc(2, 1),
		];
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.filters = vec![Filter::Gui];
		});
		let rows = d.rows();
		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].pid, 1);
	}

	#[test]
	fn rows_filter_mode_and() {
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
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.filters = vec![Filter::Kernel, Filter::Parent];
			s.filter_mode = FilterMode::And;
		});
		let rows = d.rows();
		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].pid, 1);
	}

	#[test]
	fn rows_filter_mode_or() {
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
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.filters = vec![Filter::Gui, Filter::User];
			s.filter_mode = FilterMode::Or;
		});
		let rows = d.rows();
		assert_eq!(rows.len(), 2);
	}

	#[test]
	fn rows_pid_filter_pins_to_top() {
		let procs = vec![
			make_proc(50, 1),
			make_proc(10, 1),
			make_proc(90, 100),
			make_proc(100, 1),
		];
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.filters = vec![Filter::Pid(100)];
			s.pid_filter_mode = PidFilterMode::AllDescendants;
		});
		let rows = d.rows();
		assert!(!rows.is_empty());
		assert_eq!(rows[0].pid, 100);
	}

	// ── pid scope ────────────────────────────────────────────

	#[test]
	fn rows_pid_scope_all_descendants() {
		let procs = vec![
			make_proc(1, 0),
			make_proc(10, 1),
			make_proc(100, 10),
			make_proc(50, 2),
		];
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.filters = vec![Filter::Pid(1)];
			s.pid_filter_mode = PidFilterMode::AllDescendants;
		});
		let pids: Vec<i32> = d.rows().iter().map(|p| p.pid).collect();
		assert_eq!(pids, vec![1, 10, 100]);
	}

	#[test]
	fn rows_pid_scope_direct_children() {
		let procs =
			vec![make_proc(1, 0), make_proc(10, 1), make_proc(100, 10)];
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.filters = vec![Filter::Pid(1)];
			s.pid_filter_mode = PidFilterMode::DirectChildren;
		});
		let pids: Vec<i32> = d.rows().iter().map(|p| p.pid).collect();
		assert_eq!(pids, vec![1, 10]);
	}

	#[test]
	fn rows_pid_scope_wins_over_or() {
		let procs = vec![
			{
				let mut p = make_proc(1, 0);
				p.is_gui = true;
				p
			},
			make_proc(10, 1),
			{
				let mut p = make_proc(50, 2);
				p.is_gui = true;
				p
			},
		];
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.filters = vec![Filter::Pid(1), Filter::Gui];
			s.filter_mode = FilterMode::Or;
			s.pid_filter_mode = PidFilterMode::AllDescendants;
		});
		let matched = d.match_set();
		assert!(matched.contains(&50));
		let pids: Vec<i32> = d.rows().iter().map(|p| p.pid).collect();
		assert_eq!(pids, vec![1]);
	}

	#[test]
	fn tree_match_set_ignores_pid_filter_mode() {
		let procs =
			vec![make_proc(1, 0), make_proc(10, 1), make_proc(100, 10)];
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.filters = vec![Filter::Pid(1)];
			s.pid_filter_mode = PidFilterMode::DirectChildren;
		});
		let rows_pids: Vec<i32> = d.rows().iter().map(|p| p.pid).collect();
		assert_eq!(rows_pids, vec![1, 10]);
		let tree_pids = d.tree_match_set();
		assert!(tree_pids.contains(&1));
		assert!(tree_pids.contains(&10));
		assert!(tree_pids.contains(&100));
	}

	#[test]
	fn match_set_unrestricted_without_pin() {
		let procs = vec![
			make_proc(1, 0),
			{
				let mut p = make_proc(2, 0);
				p.is_gui = true;
				p
			},
			make_proc(3, 0),
		];
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.filters = vec![Filter::Gui];
		});
		let matched = d.match_set();
		assert_eq!(matched.len(), 1);
		assert!(matched.contains(&2));
		assert_eq!(d.rows().len(), 1);
	}

	#[test]
	fn rows_cumulative_view_mode() {
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
		let d = make_engine(procs);
		set_view_state(&d, |s| {
			s.sort_col = SortColumn::Cpu;
			s.sort_dir = SortDirection::Descending;
			s.resource_view_mode = ResourceViewMode::Cumulative;
		});
		let rows = d.rows();
		assert_eq!(rows.len(), 2);
		assert_eq!(rows[0].pid, 1);
		assert_eq!(rows[1].pid, 2);
	}

	#[test]
	fn rows_cache_invalidation_on_generation() {
		let procs = vec![make_proc(1, 0), make_proc(2, 0)];
		let d = make_engine(procs.clone());
		set_view_state(&d, |s| {
			s.sort_col = SortColumn::Pid;
			s.sort_dir = SortDirection::Ascending;
		});
		let r1 = d.rows();
		assert_eq!(r1[0].pid, 1);
		d.view_state.borrow_mut().generation += 1;
		let r2 = d.rows();
		assert_eq!(r2[0].pid, 1);
	}

	fn make_proc_named(pid: i32, name: &str) -> ProcessSnapshot {
		let mut p = make_proc(pid, 0);
		p.name = name.into();
		p
	}

	fn make_proc_with_user(
		pid: i32,
		user: &str,
		is_owned: bool,
	) -> ProcessSnapshot {
		let mut p = make_proc(pid, 0);
		p.user = user.into();
		p.is_owned_by_current_user = is_owned;
		p
	}

	fn make_proc_with_cpu(pid: i32, cpu: f32) -> ProcessSnapshot {
		let mut p = make_proc(pid, 0);
		p.cpu_percent = cpu;
		p
	}
}
