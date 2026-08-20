use crate::fuzzy::best_fuzzy_score;
use crate::model::*;

fn make_process(
	pid: i32,
	ppid: i32,
	name: &str,
	user: &str,
	state: char,
	command: &str,
) -> ProcessSnapshot {
	ProcessSnapshot {
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
	assert!(best_fuzzy_score("firefox", &p, &mut m).1 > 0);
}

#[test]
fn best_score_zero_when_no_match() {
	let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
	let p = make_process(1, 0, "bash", "root", 'S', "bash");
	assert_eq!(best_fuzzy_score("xyzzy", &p, &mut m), (0, 0));
}

#[test]
fn best_score_uses_electron_app_name() {
	let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
	let mut p = make_process(1, 0, "electron", "alice", 'S', "electron");
	p.electron_app_name = Some("discord".into());
	assert!(best_fuzzy_score("discord", &p, &mut m).0 > 0);
}

#[test]
fn best_score_name_outranks_command() {
	let mut m = nucleo::Matcher::new(nucleo::Config::DEFAULT);
	// gpuitop's own cmdline carries "--search vesktop", so it matches the
	// command; the real vesktop process matches by its displayed name. The
	// name match must rank first, not fall through to the CPU tiebreak.
	let gpuitop = make_process(
		1,
		0,
		"gpuitop",
		"alice",
		'S',
		"gpuitop --search vesktop",
	);
	let mut vesktop =
		make_process(2, 0, "electron", "alice", 'S', "electron");
	vesktop.electron_app_name = Some("vesktop".into());
	assert!(
		best_fuzzy_score("vesktop", &vesktop, &mut m)
			> best_fuzzy_score("vesktop", &gpuitop, &mut m)
	);
}
