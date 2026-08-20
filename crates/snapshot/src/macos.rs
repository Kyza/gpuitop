use gpuitop_core::model::SystemSnapshot;

use super::CollectorState;

pub fn collect_snapshot(_state: &mut CollectorState) -> SystemSnapshot {
	SystemSnapshot::empty()
}
