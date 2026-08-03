use gpui::{px, SharedString};
use gpui_component::theme::{Theme, ThemeConfig, ThemeRegistry};
use std::rc::Rc;

pub fn apply_theme_by_name(
	name: &SharedString,
	window: Option<&mut gpui::Window>,
	cx: &mut gpui::App,
) {
	{
		let registry = ThemeRegistry::global(cx);
		let themes = registry.themes();

		if let Some(config) = themes.get(name) {
			let mode = config.mode;
			let config_rc = config.clone();
			let sibling = if mode.is_dark() {
				find_sibling(themes, name, false)
			} else {
				find_sibling(themes, name, true)
			};

			let theme = Theme::global_mut(cx);
			if mode.is_dark() {
				theme.dark_theme = config_rc;
			} else {
				theme.light_theme = config_rc;
			}
			if let Some(sib) = sibling {
				if sib.mode.is_dark() {
					theme.dark_theme = sib;
				} else {
					theme.light_theme = sib;
				}
			}
			Theme::change(mode, window, cx);
			let theme = Theme::global_mut(cx);
			let active = if mode.is_dark() {
				theme.dark_theme.clone()
			} else {
				theme.light_theme.clone()
			};
			if active.radius.is_none() {
				theme.radius = px(6.0);
			}
			if active.radius_lg.is_none() {
				theme.radius_lg = px(8.0);
			}
			return;
		}
	}

	let fallback = {
		let registry = ThemeRegistry::global(cx);
		registry
			.themes()
			.get(&SharedString::from("Default Dark"))
			.cloned()
	};
	if let Some(default_dark) = fallback {
		let theme = Theme::global_mut(cx);
		theme.dark_theme = default_dark;
		Theme::change(gpui_component::ThemeMode::Dark, window, cx);
		return;
	}

	Theme::change(gpui_component::ThemeMode::Dark, window, cx);
}

fn find_sibling(
	themes: &std::collections::HashMap<SharedString, Rc<ThemeConfig>>,
	name: &SharedString,
	want_dark: bool,
) -> Option<Rc<ThemeConfig>> {
	let wanted_mode = if want_dark {
		gpui_component::ThemeMode::Dark
	} else {
		gpui_component::ThemeMode::Light
	};
	let family = theme_family_of(name);
	for (candidate_name, config) in themes.iter() {
		let c_family = theme_family_of(candidate_name);
		if c_family == family
			&& candidate_name != name
			&& config.mode == wanted_mode
		{
			return Some(config.clone());
		}
	}
	None
}

fn theme_family_of(name: &str) -> &str {
	name.rsplit_once(' ').map(|(f, _)| f).unwrap_or(name)
}

#[derive(Debug, Clone)]
pub struct ThemeFamily {
	pub name: SharedString,
	pub variants: Vec<(SharedString, gpui_component::ThemeMode)>,
}

pub fn list_theme_families(cx: &gpui::App) -> Vec<ThemeFamily> {
	let registry = ThemeRegistry::global(cx);
	let mut families: std::collections::BTreeMap<
		String,
		Vec<(SharedString, gpui_component::ThemeMode)>,
	> = std::collections::BTreeMap::new();

	for config in registry.sorted_themes().iter() {
		let family_name = config
			.name
			.rsplit_once(' ')
			.map(|(f, _)| f)
			.unwrap_or(&config.name)
			.to_string();
		families
			.entry(family_name)
			.or_default()
			.push((config.name.clone(), config.mode));
	}

	families
		.into_iter()
		.map(|(name, variants)| ThemeFamily {
			name: SharedString::from(name),
			variants,
		})
		.collect()
}
