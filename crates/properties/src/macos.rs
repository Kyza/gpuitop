use super::{Platform, ProcessProperties, PropertiesInterface};

impl_interface! {
	fn collect(_pid: i32) -> Option<ProcessProperties> {
		None
	}
}
