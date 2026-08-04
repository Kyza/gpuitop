use crate::data::icons::DesktopEntryCache;
use crate::data::model::{GpuBackend, SystemSnapshot};
use std::collections::HashMap;
use std::sync::Arc;

pub struct CollectorState {
	pub prev_cpu: Option<()>,
	pub gpu_backend: GpuBackend,
}

impl CollectorState {
	pub fn new(
		gpu_backend: GpuBackend,
		_desktop_cache: Arc<DesktopEntryCache>,
	) -> Self {
		Self {
			prev_cpu: None,
			gpu_backend,
		}
	}
}

impl SystemSnapshot {
	pub fn new(state: &mut CollectorState) -> Self {
		SystemSnapshot::empty()
	}
}

pub fn pid_alive(_pid: i32) -> bool {
	false
}
