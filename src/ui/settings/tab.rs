use crate::data::config::Config;
use crate::data::model::*;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	button::{Button, ButtonVariants},
	setting::{SettingPage, Settings},
	ActiveTheme, Icon, IconName, Sizable,
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

pub struct SettingsTab {
	pub config: Config,
	refresh_ms: Arc<AtomicU64>,
	theme: Rc<Cell<Theme>>,
}

impl SettingsTab {
	pub fn new(
		config: Config,
		refresh_ms: Arc<AtomicU64>,
		theme: Rc<Cell<Theme>>,
		_cx: &mut Context<Self>,
	) -> Self {
		Self {
			config,
			refresh_ms,
			theme,
		}
	}

	pub fn save(&self) {
		if let Err(e) = self.config.save() {
			eprintln!("Failed to save config: {e}");
		}
	}

	fn setting_pages(
		&self,
		_window: &mut Window,
		cx: &mut Context<Self>,
	) -> Vec<SettingPage> {
		let view = cx.entity();
		let refresh_ms = self.refresh_ms.clone();
		let theme_cell = self.theme.clone();
		let default_config = Config::default();

		vec![
			super::general::general_page(
				&view,
				&refresh_ms,
				&theme_cell,
				&default_config,
			),
			super::processes::processes_page(&view, &default_config),
			super::about::about_page(),
		]
	}
}

impl Render for SettingsTab {
	fn render(
		&mut self,
		window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		div()
			.size_full()
			.flex()
			.flex_col()
			.child(
				div().flex_grow(1.0).w_full().overflow_hidden().child(
					Settings::new("gpuitop-settings")
						.pages(self.setting_pages(window, cx)),
				),
			)
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
								Icon::new(IconName::ExternalLink)
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
