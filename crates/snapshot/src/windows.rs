use super::{CollectorState, Platform, SnapshotInterface};
use gpuitop_core::model::SystemSnapshot;

impl_interface! {
	fn collect_snapshot(_state: &mut CollectorState) -> SystemSnapshot {
		SystemSnapshot::empty()
	}

	fn pid_alive(_pid: i32) -> bool {
		false
	}
}
