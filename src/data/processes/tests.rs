use crate::data::fuzzy::best_fuzzy_score;
use crate::data::model::*;

fn make_process(
	pid: i32,
	ppid: i32,
	name: &str,
	user: &str,
	state: char,
	command: &str,
) -> ProcessInfo {
	ProcessInfo {
		pid,
		ppid,
		name: name.into(),
		user: user.into(),
		state,
		command: command.into(),
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
		icon_name: None,
		has_children: false,
	}
}

fn is_descendant_of(
	pid: i32,
	ancestor: i32,
	processes: &[ProcessInfo],
) -> bool {
	let mut current = pid;
	for _ in 0..100 {
		if current == ancestor {
			return true;
		}
		if let Some(p) = processes.iter().find(|p| p.pid == current) {
			current = p.ppid;
		} else {
			break;
		}
	}
	false
}

fn proc_matches_standalone(
	proc: &ProcessInfo,
	filter: &Filter,
	processes: &[ProcessInfo],
	pid_filter_mode: PidFilterMode,
) -> bool {
	match filter {
		Filter::Gui => proc.is_gui,
		Filter::User => proc.is_owned_by_current_user && !proc.is_gui,
		Filter::System => {
			!proc.is_kthread
				&& !proc.is_owned_by_current_user
				&& proc.ppid != 1
		}
		// Test-only simplification — real impl in delegate.rs
		// uses is_service() with init-system detection.
		Filter::Services => proc.ppid == 1,
		Filter::Kernel => proc.is_kthread,
		Filter::Parent => proc.has_children,
		Filter::Vram => proc.vram_bytes.is_some(),
		Filter::Electron => proc.is_electron,
		Filter::ProcessState(c) => proc.state == *c,
		Filter::Username(s) => proc.user == *s,
		Filter::Pid(pid) => match pid_filter_mode {
			PidFilterMode::AllDescendants => {
				proc.pid == *pid
					|| is_descendant_of(proc.pid, *pid, processes)
			}
			PidFilterMode::DirectChildren => {
				proc.pid == *pid || proc.ppid == *pid
			}
		},
	}
}

// ── best_fuzzy_score ─────────────────────────────────────

#[test]
fn best_score_exact_ranks_higher() {
	let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
	let exact = make_process(1, 0, "firefox", "alice", 'S', "firefox");
	let partial = make_process(2, 0, "foxhelper", "alice", 'S', "foxhelper");
	assert!(
		best_fuzzy_score("firefox", &exact, &mut m)
			> best_fuzzy_score("firefox", &partial, &mut m)
	);
}

#[test]
fn best_score_matches_command() {
	let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
	let p = make_process(1, 0, "bash", "root", 'S', "/usr/bin/firefox-esr");
	assert!(best_fuzzy_score("firefox", &p, &mut m) > 0);
}

#[test]
fn best_score_zero_when_no_match() {
	let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
	let p = make_process(1, 0, "bash", "root", 'S', "bash");
	assert_eq!(best_fuzzy_score("xyzzy", &p, &mut m), 0);
}

#[test]
fn best_score_uses_electron_app_name() {
	let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
	let mut p = make_process(1, 0, "electron", "alice", 'S', "electron");
	p.electron_app_name = Some("discord".into());
	assert!(best_fuzzy_score("discord", &p, &mut m) > 0);
}

// ── filter matching ──────────────────────────────────────

#[test]
fn filter_gui() {
	let p = make_process(1, 0, "app", "alice", 'S', "app");
	let mut pg = p.clone();
	pg.is_gui = true;
	assert!(!proc_matches_standalone(
		&p,
		&Filter::Gui,
		&[],
		PidFilterMode::AllDescendants
	));
	assert!(proc_matches_standalone(
		&pg,
		&Filter::Gui,
		&[],
		PidFilterMode::AllDescendants
	));
}

#[test]
fn filter_user() {
	let mut p = make_process(1, 0, "app", "alice", 'S', "app");
	p.is_owned_by_current_user = true;
	assert!(proc_matches_standalone(
		&p,
		&Filter::User,
		&[],
		PidFilterMode::AllDescendants
	));
}

#[test]
fn filter_user_excludes_gui() {
	let mut p = make_process(1, 0, "app", "alice", 'S', "app");
	p.is_owned_by_current_user = true;
	p.is_gui = true;
	assert!(!proc_matches_standalone(
		&p,
		&Filter::User,
		&[],
		PidFilterMode::AllDescendants
	));
}

#[test]
fn filter_system() {
	let p = make_process(2, 100, "sshd", "root", 'S', "sshd");
	assert!(proc_matches_standalone(
		&p,
		&Filter::System,
		&[],
		PidFilterMode::AllDescendants
	));
}
#[test]
fn filter_system_excludes_kthread() {
	let mut p = make_process(2, 100, "kworker", "root", 'S', "kworker");
	p.is_kthread = true;
	assert!(!proc_matches_standalone(
		&p,
		&Filter::System,
		&[],
		PidFilterMode::AllDescendants
	));
}

#[test]
fn filter_services() {
	let p = make_process(100, 1, "sshd", "root", 'S', "sshd");
	assert!(proc_matches_standalone(
		&p,
		&Filter::Services,
		&[],
		PidFilterMode::AllDescendants
	));
}

#[test]
fn filter_kernel() {
	let mut p = make_process(2, 0, "kthreadd", "root", 'S', "kthreadd");
	p.is_kthread = true;
	assert!(proc_matches_standalone(
		&p,
		&Filter::Kernel,
		&[],
		PidFilterMode::AllDescendants
	));
}

#[test]
fn filter_parent() {
	let mut p = make_process(1, 0, "systemd", "root", 'S', "systemd");
	p.has_children = true;
	assert!(proc_matches_standalone(
		&p,
		&Filter::Parent,
		&[],
		PidFilterMode::AllDescendants
	));
	assert!(!proc_matches_standalone(
		&make_process(2, 1, "child", "root", 'S', "child"),
		&Filter::Parent,
		&[],
		PidFilterMode::AllDescendants,
	));
}

#[test]
fn filter_vram() {
	let mut p = make_process(1, 0, "gpu_app", "alice", 'S', "gpu_app");
	p.vram_bytes = Some(1024 * 1024);
	assert!(proc_matches_standalone(
		&p,
		&Filter::Vram,
		&[],
		PidFilterMode::AllDescendants
	));
	assert!(!proc_matches_standalone(
		&make_process(2, 0, "no_gpu", "alice", 'S', "no_gpu"),
		&Filter::Vram,
		&[],
		PidFilterMode::AllDescendants,
	));
}

#[test]
fn filter_electron() {
	let mut p = make_process(1, 0, "electron", "alice", 'S', "electron");
	p.is_electron = true;
	assert!(proc_matches_standalone(
		&p,
		&Filter::Electron,
		&[],
		PidFilterMode::AllDescendants
	));
}

#[test]
fn filter_state() {
	let running = make_process(1, 0, "app", "alice", 'R', "app");
	let sleeping = make_process(2, 0, "app", "alice", 'S', "app");
	assert!(proc_matches_standalone(
		&running,
		&Filter::ProcessState('R'),
		&[],
		PidFilterMode::AllDescendants,
	));
	assert!(!proc_matches_standalone(
		&sleeping,
		&Filter::ProcessState('R'),
		&[],
		PidFilterMode::AllDescendants,
	));
}

#[test]
fn filter_pid_direct_child() {
	let child = make_process(200, 100, "child", "root", 'S', "child");
	assert!(proc_matches_standalone(
		&child,
		&Filter::Pid(100),
		&[],
		PidFilterMode::DirectChildren,
	));
}

#[test]
fn filter_pid_all_descendants_grandchild() {
	let grandparent = make_process(10, 1, "grand", "root", 'S', "grand");
	let parent = make_process(20, 10, "parent", "root", 'S', "parent");
	let child = make_process(30, 20, "child", "root", 'S', "child");
	let procs = vec![grandparent, parent, child.clone()];
	assert!(proc_matches_standalone(
		&child,
		&Filter::Pid(10),
		&procs,
		PidFilterMode::AllDescendants,
	));
}

#[test]
fn filter_pid_direct_children_grandchild() {
	let grandparent = make_process(10, 1, "grand", "root", 'S', "grand");
	let parent = make_process(20, 10, "parent", "root", 'S', "parent");
	let child = make_process(30, 20, "child", "root", 'S', "child");
	let procs = vec![grandparent, parent, child.clone()];
	assert!(!proc_matches_standalone(
		&child,
		&Filter::Pid(10),
		&procs,
		PidFilterMode::DirectChildren,
	));
}

#[test]
fn filter_pid_direct_child_all_descendants() {
	let parent = make_process(100, 1, "parent", "root", 'S', "parent");
	let child = make_process(200, 100, "child", "root", 'S', "child");
	let procs = vec![parent, child.clone()];
	assert!(proc_matches_standalone(
		&child,
		&Filter::Pid(100),
		&procs,
		PidFilterMode::AllDescendants,
	));
}

#[test]
fn filter_pid_descendant() {
	let grandparent = make_process(10, 1, "grand", "root", 'S', "grand");
	let parent = make_process(20, 10, "parent", "root", 'S', "parent");
	let child = make_process(30, 20, "child", "root", 'S', "child");
	let procs = vec![grandparent, parent, child.clone()];
	assert!(is_descendant_of(30, 10, &procs));
	assert!(!is_descendant_of(30, 99, &procs));
}

// ── FilterMode ───────────────────────────────────────────

#[test]
fn filter_mode_and_all_match() {
	let mut p = make_process(10, 1, "app", "root", 'S', "app");
	p.is_owned_by_current_user = true;
	let filters = vec![Filter::Services, Filter::User];
	let all = filters.iter().all(|f| {
		proc_matches_standalone(&p, f, &[], PidFilterMode::AllDescendants)
	});
	assert!(all);
}

#[test]
fn filter_mode_and_one_fails() {
	let p = make_process(10, 1, "app", "root", 'S', "app");
	let filters = vec![Filter::Services, Filter::Kernel];
	let all = filters.iter().all(|f| {
		proc_matches_standalone(&p, f, &[], PidFilterMode::AllDescendants)
	});
	assert!(!all);
}

#[test]
fn filter_mode_or_any_match() {
	let p = make_process(10, 1, "app", "root", 'S', "app");
	let filters = vec![Filter::Services, Filter::Kernel];
	let any = filters.iter().any(|f| {
		proc_matches_standalone(&p, f, &[], PidFilterMode::AllDescendants)
	});
	assert!(any);
}

#[test]
fn filter_mode_or_none_match() {
	let p = make_process(10, 0, "app", "root", 'S', "app");
	let filters = vec![Filter::Services, Filter::Kernel];
	let any = filters.iter().any(|f| {
		proc_matches_standalone(&p, f, &[], PidFilterMode::AllDescendants)
	});
	assert!(!any);
}

#[test]
fn empty_filters_pass_all() {
	let p = make_process(1, 0, "app", "root", 'S', "app");
	let filters: Vec<Filter> = vec![];
	let all = filters.iter().all(|f| {
		proc_matches_standalone(&p, f, &[], PidFilterMode::AllDescendants)
	});
	assert!(all);
	let any = filters.iter().any(|f| {
		proc_matches_standalone(&p, f, &[], PidFilterMode::AllDescendants)
	});
	assert!(!any);
}
