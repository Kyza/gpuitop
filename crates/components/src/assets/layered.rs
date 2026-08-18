use gpui::*;
use std::borrow::Cow;

pub struct LayeredAssets {
	layers: Vec<Box<dyn AssetSource + Send + Sync>>,
}

impl LayeredAssets {
	pub fn new() -> Self {
		Self { layers: Vec::new() }
	}

	pub fn with(mut self, source: impl AssetSource) -> Self {
		self.layers.push(Box::new(source));
		self
	}
}

impl AssetSource for LayeredAssets {
	fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
		for layer in self.layers.iter().rev() {
			match layer.load(path) {
				Ok(Some(data)) => return Ok(Some(data)),
				Ok(None) | Err(_) => continue,
			}
		}
		Ok(None)
	}

	fn list(&self, path: &str) -> Result<Vec<SharedString>> {
		let mut all = Vec::new();
		for layer in &self.layers {
			match layer.list(path) {
				Ok(list) => all.extend(list),
				Err(_) => continue,
			}
		}
		all.sort();
		all.dedup();
		Ok(all)
	}
}
