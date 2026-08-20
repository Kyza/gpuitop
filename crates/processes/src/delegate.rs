use std::ops::Deref;
use std::rc::Rc;

use gpuitop_core::processes::engine::ProcessEngine;

pub struct ProcessTableDelegate(pub Rc<ProcessEngine>);

impl ProcessTableDelegate {
	pub fn from_engine(engine: Rc<ProcessEngine>) -> Self {
		Self(engine)
	}
}

impl Deref for ProcessTableDelegate {
	type Target = Rc<ProcessEngine>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
