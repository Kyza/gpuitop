use rust_embed::RustEmbed;
use std::sync::OnceLock;

#[derive(RustEmbed)]
#[folder = "src/data/themes/"]
#[include = "*.json"]
struct ThemeAssets;

fn builtin_theme_names() -> &'static [String] {
	static NAMES: OnceLock<Vec<String>> = OnceLock::new();
	NAMES.get_or_init(|| {
		ThemeAssets::iter()
			.filter_map(|filename| {
				let file = ThemeAssets::get(&filename)?;
				let v: serde_json::Value =
					serde_json::from_slice(&file.data).ok()?;
				v["themes"][0]["name"].as_str().map(String::from)
			})
			.collect()
	})
}

pub fn register_builtin_themes(cx: &mut gpui::App) {
	let any_registered = {
		let registry = gpui_component::theme::ThemeRegistry::global(cx);
		builtin_theme_names()
			.iter()
			.any(|name| registry.themes().contains_key(name.as_str()))
	};
	if any_registered {
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
