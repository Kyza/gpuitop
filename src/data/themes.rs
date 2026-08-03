use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/data/themes/"]
#[include = "*.json"]
struct ThemeAssets;

pub fn unpack_builtins_to_disk() {
	let themes_dir = crate::data::config::Config::config_path()
		.parent()
		.map(|p| p.to_path_buf())
		.unwrap_or_default()
		.join("themes");
	let _ = std::fs::create_dir_all(&themes_dir);

	for filename in ThemeAssets::iter() {
		let dest = themes_dir.join(filename.as_ref());
		if !dest.exists() {
			if let Some(file) = ThemeAssets::get(&filename) {
				let _ = std::fs::write(&dest, &file.data);
			}
		}
	}
}

pub fn load_builtins_into_registry(cx: &mut gpui::App) {
	let registry = gpui_component::theme::ThemeRegistry::global_mut(cx);
	for filename in ThemeAssets::iter() {
		if let Some(file) = ThemeAssets::get(&filename) {
			let json = std::str::from_utf8(&file.data).unwrap_or_default();
			let _ = registry.load_themes_from_str(json);
		}
	}
}
