use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/data/themes/"]
struct ThemeAssets;

pub fn register_builtin_themes(cx: &mut gpui::App) {
	let registry = gpui_component::theme::ThemeRegistry::global_mut(cx);
	if registry.themes().contains_key("Catppuccin Latte") {
		return;
	}
	for filename in ThemeAssets::iter() {
		if !filename.ends_with(".json") {
			continue;
		}
		if let Some(file) = ThemeAssets::get(&filename) {
			let json = std::str::from_utf8(&file.data).unwrap_or_default();
			let _ = registry.load_themes_from_str(json);
		}
	}
}
