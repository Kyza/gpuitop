use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use gpuitop_core::processes::delegate::ProcessTableDelegate as CoreDelegate;
use gpuitop_core::processes::engine::ProcessEngine;

pub struct ProcessTableDelegate(pub CoreDelegate);

impl ProcessTableDelegate {
	pub fn from_engine(engine: Rc<RefCell<ProcessEngine>>) -> Self {
		Self(CoreDelegate::from_engine(engine))
	}
}

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
