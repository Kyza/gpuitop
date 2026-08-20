use crate::config::Config;
use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

#[derive(Clone)]
pub struct ConfigStore {
	inner: Rc<RefCell<Config>>,
	generation: Rc<Cell<u64>>,
}

impl ConfigStore {
	pub fn new(config: Config) -> Self {
		Self {
			inner: Rc::new(RefCell::new(config)),
			generation: Rc::new(Cell::new(0)),
		}
	}

	pub fn get(&self) -> Ref<'_, Config> {
		self.inner.borrow()
	}

	pub fn generation(&self) -> u64 {
		self.generation.get()
	}

	/// Mutates the shared config, bumps the generation counter, and persists.
	/// The single write path for every config change in the app.
	pub fn mutate(&self, f: impl FnOnce(&mut Config)) {
		{
			let mut cfg = self.inner.borrow_mut();
			f(&mut cfg);
		}
		self.generation.set(self.generation.get().wrapping_add(1));
		if let Err(e) = self.inner.borrow().save() {
			eprintln!("Failed to save config: {e}");
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::model::DefaultViewMode;

	fn temp_config_home() -> std::path::PathBuf {
		let dir = std::env::temp_dir()
			.join(format!("gpuitop-configstore-test-{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&dir);
		dir
	}

	#[test]
	fn mutate_applies_and_bumps_generation() {
		let store = ConfigStore::new(Config::default());
		assert_eq!(store.generation(), 0);
		store.mutate(|c| {
			c.general.interface.refresh_ms = 500;
		});
		assert_eq!(store.generation(), 1);
		assert_eq!(store.get().general.interface.refresh_ms, 500);
	}

	#[test]
	fn mutate_persists_to_disk() {
		std::env::set_var("XDG_CONFIG_HOME", temp_config_home());
		let store = ConfigStore::new(Config::default());
		store.mutate(|c| {
			c.processes.behaviour.default_view_mode = DefaultViewMode::Tree;
		});
		let data = std::fs::read_to_string(Config::config_path())
			.expect("config written");
		let loaded: Config = ron::from_str(&data).expect("parses");
		assert_eq!(
			loaded.processes.behaviour.default_view_mode,
			DefaultViewMode::Tree
		);
	}

	#[test]
	fn clone_shares_state() {
		let store = ConfigStore::new(Config::default());
		let other = store.clone();
		other.mutate(|c| c.window_size = (800, 600));
		assert_eq!(store.get().window_size, (800, 600));
		assert_eq!(store.generation(), other.generation());
	}
}
