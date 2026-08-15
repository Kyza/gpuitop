use std::ops::{Deref, DerefMut};

use gpuitop_core::processes::delegate::ProcessTableDelegate as CoreDelegate;

pub struct ProcessTableDelegate(pub CoreDelegate);

impl Deref for ProcessTableDelegate {
	type Target = CoreDelegate;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for ProcessTableDelegate {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}
