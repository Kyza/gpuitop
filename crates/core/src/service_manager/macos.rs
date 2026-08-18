use std::collections::HashSet;

use super::InitSystem;
use crate::model::ProcessSnapshot;

pub fn detect_init() -> InitSystem {
	InitSystem::Unknown
}

pub fn pids_of_runsv(_processes: &[ProcessSnapshot]) -> HashSet<i32> {
	HashSet::new()
}

pub fn pids_of_supervise_daemon(
	_processes: &[ProcessSnapshot],
) -> HashSet<i32> {
	HashSet::new()
}

pub fn is_service(
	_proc: &ProcessSnapshot,
	_init: InitSystem,
	_runsv_pids: &HashSet<i32>,
	_supervise_pids: &HashSet<i32>,
) -> bool {
	false
}
