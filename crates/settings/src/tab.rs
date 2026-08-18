use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	button::{Button, ButtonVariants},
	setting::{SelectIndex, SettingPage, Settings},
	ActiveTheme, Sizable,
};
use gpuitop_components::assets::lucide::LucideIcon;
use gpuitop_core::about::DepInfo;
use gpuitop_core::config::Config;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

pub struct SettingsTab {
	pub config: Config,
	refresh_ms: Arc<AtomicU64>,
	initial_page_index: Option<usize>,
	deps: &'static [DepInfo],
}

impl SettingsTab {
	pub fn new(
		config: Config,
		refresh_ms: Arc<AtomicU64>,
		initial_page_index: Option<usize>,
		deps: &'static [DepInfo],
		_cx: &mut Context<Self>,
	) -> Self {
		Self {
			config,
			refresh_ms,
			initial_page_index,
			deps,
		}
	}

	pub fn save(&self) {
		if let Err(e) = self.config.save() {
			eprintln!("Failed to save config: {e}");
		}
	}

	#[hotpath::measure]
	fn setting_pages(
		&self,
		_window: &mut Window,
		cx: &mut Context<Self>,
	) -> Vec<SettingPage> {
		let view = cx.entity();
		let refresh_ms = self.refresh_ms.clone();
		let default_config = Config::default();

		vec![
			crate::general::general_page(&view, &refresh_ms, &default_config),
			crate::processes::processes_page(&view, &default_config),
			crate::about::about_page(self.deps),
		]
	}
}

impl Render for SettingsTab {
	#[hotpath::measure]
	fn render(
		&mut self,
		window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		div()
			.size_full()
			.flex()
			.flex_col()
			.child(div().flex_grow(1.0).w_full().overflow_hidden().child({
				let mut settings = Settings::new("gpuitop-settings")
					.pages(self.setting_pages(window, cx));
				if let Some(ix) = self.initial_page_index {
					settings = settings.default_selected_index(SelectIndex {
						page_ix: ix,
						group_ix: None,
					});
				}
				settings
			}))
			.child(
				div()
					.w_full()
					.h(px(32.0))
					.flex()
					.flex_row()
					.items_center()
					.justify_end()
					.px(px(8.0))
					.border_t_1()
					.border_color(cx.theme().border)
					.bg(cx.theme().background)
					.child(
						Button::new("open-config")
							.ghost()
							.label("Open Config File")
							.icon(
								LucideIcon::ExternalLink
									.icon()
									.size(px(12.0))
									.text_color(cx.theme().muted_foreground),
							)
							.xsmall()
							.on_click(|_, _, _| {
								let path = Config::config_path();
								let _ = open::that(path);
							}),
					),
			)
			.into_any_element()
	}
}
