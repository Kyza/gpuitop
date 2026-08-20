use gpuitop_snapshot::ReadError;

use super::ProcessProperties;

pub fn collect(_pid: i32) -> Result<ProcessProperties, ReadError> {
	Err(ReadError::Dead)
}
