mod general;
mod processes;
mod columns;
mod about;

use crate::config::Config;
use crate::model::*;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	setting::{SettingPage, Settings},
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

pub struct SettingsTab {
	config: Config,
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

	fn save(&self) {
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
			general::general_page(&view, &refresh_ms, &theme_cell, &default_config),
			processes::processes_page(&view, &default_config),
			columns::columns_page(&view, &default_config),
			about::about_page(),
		]
	}
}

impl Render for SettingsTab {
	fn render(
		&mut self,
		window: &mut Window,
		cx: &mut Context<Self>,
	) -> impl IntoElement {
		Settings::new("gpuitop-settings").pages(self.setting_pages(window, cx))
	}
}
