use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/data/themes/"]
#[include = "*.json"]
struct ThemeAssets;

pub fn register_builtin_themes(cx: &mut gpui::App) {
	let already_registered = gpui_component::theme::ThemeRegistry::global(cx)
		.themes()
		.contains_key("Catppuccin Latte");
	if already_registered {
		return;
	}
	let registry = gpui_component::theme::ThemeRegistry::global_mut(cx);
	for filename in ThemeAssets::iter() {
		if let Some(file) = ThemeAssets::get(&filename) {
			let json = std::str::from_utf8(&file.data).unwrap_or_default();
			let _ = registry.load_themes_from_str(json);
		}
	}
}
