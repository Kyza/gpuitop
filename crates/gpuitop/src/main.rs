mod app_view;
mod cli;

#[allow(dead_code)]
mod built {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

use gpui::*;
use gpui_component::Root;
use gpuitop_components::assets::{layered, lucide};
use gpuitop_components::themes;
use gpuitop_core::config::Config;
use gpuitop_core::config_store::ConfigStore;
use gpuitop_icons::load_cache;
use gpuitop_properties::PropertiesWindow;
use gpuitop_window_picker::GPUITOP_APP_ID;
use std::sync::Arc;

#[hotpath::main]
fn main() {
	let cli = cli::parse();

	let mut config = Config::load_from(cli.config_path.as_deref());

	for ov in &cli.overrides {
		if let Err(e) = config.apply_override(ov) {
			eprintln!("--override error: {e}");
		}
	}

	let active_tab = cli.active_tab();
	let settings_page = cli.settings_page_index();
	let performance_tab = cli.performance_tab_index();
	let search = cli.search.clone();
	let override_view = cli.override_view();

	let (mut win_width, mut win_height) = config.window_size;
	if win_width < 640 {
		win_width = 640;
	}
	if win_height < 400 {
		win_height = 400;
	}
	let win_width = win_width as f32;
	let win_height = win_height as f32;

	let app = gpui_platform::application().with_assets(
		layered::LayeredAssets::new()
			.with(gpui_component_assets::Assets)
			.with(lucide::LucideAssets),
	);
	let desktop_cache = Arc::new(load_cache());
	app.run(move |cx: &mut App| {
		gpui_component::init(cx);
		themes::unpack_builtins_to_disk();
		themes::load_builtins_into_registry(cx);

		let config_dir = Config::config_path()
			.parent()
			.map(|p| p.to_path_buf())
			.unwrap_or_default();
		let _ = gpui_component::theme::ThemeRegistry::watch_dir(
			config_dir.join("themes"),
			cx,
			|_| {},
		);

		if let Some(pid) = cli.properties_pid {
			if !gpui_component::theme::ThemeRegistry::global(cx)
				.themes()
				.contains_key(config.general.interface.theme.as_str())
			{
				config.general.interface.theme = "Default Dark".to_string();
			}

			let window = cx
				.open_window(
					WindowOptions {
						window_bounds: Some(WindowBounds::Windowed(Bounds {
							origin: point(px(100.0), px(100.0)),
							size: size(px(win_width), px(win_height)),
						})),
						window_min_size: Some(size(px(480.0), px(320.0))),
						titlebar: Some(TitlebarOptions {
							title: Some(SharedString::new(format!(
								"Properties — PID {pid}"
							))),
							appears_transparent: true,
							..Default::default()
						}),
						window_decorations: Some(WindowDecorations::Client),
						app_id: Some(
							cli.app_id
								.clone()
								.unwrap_or_else(|| GPUITOP_APP_ID.to_string())
								.into(),
						),
						..Default::default()
					},
					move |window, cx| {
						themes::apply_theme_by_name(
							&config.general.interface.theme,
							Some(window),
							cx,
						);
						let view =
							cx.new(|cx| PropertiesWindow::new(pid, None, cx));
						cx.new(|cx| Root::new(view, window, cx))
					},
				)
				.expect("Failed to open properties window");
			cx.on_window_closed(move |cx, closed| {
				if closed == window.window_id() {
					cx.quit();
				}
			})
			.detach();
			return;
		}

		let window = cx
			.open_window(
				WindowOptions {
					window_bounds: Some(WindowBounds::Windowed(Bounds {
						origin: point(px(100.0), px(100.0)),
						size: size(px(win_width), px(win_height)),
					})),
					window_min_size: Some(size(px(640.0), px(400.0))),
					titlebar: Some(TitlebarOptions {
						title: Some(SharedString::new("GPUI Top")),
						appears_transparent: true,
						..Default::default()
					}),
					window_decorations: Some(WindowDecorations::Client),
					app_id: Some(
						cli.app_id
							.clone()
							.unwrap_or_else(|| GPUITOP_APP_ID.to_string())
							.into(),
					),
					..Default::default()
				},
				|window, cx| {
					if !gpui_component::theme::ThemeRegistry::global(cx)
						.themes()
						.contains_key(config.general.interface.theme.as_str())
					{
						config.general.interface.theme =
							"Default Dark".to_string();
						let _ = config.save();
					}

					let view = cx.new(|cx| {
						app_view::App::new(
							active_tab,
							settings_page,
							performance_tab,
							search,
							override_view,
							ConfigStore::new(config.clone()),
							desktop_cache.clone(),
							cx,
						)
					});
					let theme_name = view
						.read(cx)
						.config
						.get()
						.general
						.interface
						.theme
						.clone();
					themes::apply_theme_by_name(
						&theme_name,
						Some(window),
						cx,
					);
					cx.new(|cx| Root::new(view, window, cx))
				},
			)
			.expect("Failed to open GPUI window");
		cx.on_window_closed(move |cx, closed| {
			if closed == window.window_id() {
				cx.quit();
			}
		})
		.detach();
	});
}
