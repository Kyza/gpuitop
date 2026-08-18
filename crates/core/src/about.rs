#[derive(Clone)]
pub struct DepInfo {
	pub name: &'static str,
	pub version: &'static str,
	pub license: &'static str,
	pub repository: &'static str,
	pub homepage: &'static str,
	pub description: &'static str,
}
