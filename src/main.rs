#![recursion_limit = "512"]
mod cli;
mod data;
mod ui;

#[allow(dead_code)]
mod built {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

use crate::ui::app::app_view;
use crate::ui::assets::{layered, lucide};
use gpui::*;
use gpui_component::Root;

pub const GPUITOP_APP_ID: &str = "com.github.kyza.gpuitop";

#[hotpath::main]
fn main() {
	let cli = cli::parse();

	let mut config =
		crate::data::config::Config::load_from(cli.config_path.as_deref());

	for ov in &cli.overrides {
		if let Err(e) = config.apply_override(ov) {
			eprintln!("--override error: {e}");
		}
	}

	let active_tab = cli.active_tab();
	let settings_page = cli.settings_page_index();
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
	app.run(move |cx: &mut App| {
		gpui_component::init(cx);
		crate::data::themes::unpack_builtins_to_disk();
		crate::data::themes::load_builtins_into_registry(cx);

		let config_dir = crate::data::config::Config::config_path()
			.parent()
			.map(|p| p.to_path_buf())
			.unwrap_or_default();
		let _ = gpui_component::theme::ThemeRegistry::watch_dir(
			config_dir.join("themes"),
			cx,
			|_| {},
		);

		cx.open_window(
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
				app_id: Some(GPUITOP_APP_ID.into()),
				..Default::default()
			},
			|window, cx| {
				if !gpui_component::theme::ThemeRegistry::global(cx)
					.themes()
					.contains_key(&config.general.interface.theme)
				{
					config.general.interface.theme =
						SharedString::new_static("Default Dark");
					let _ = config.save();
				}

				let view = cx.new(|cx| {
					app_view::App::new(
						active_tab,
						settings_page,
						search,
						override_view,
						config.clone(),
						cx,
					)
				});
				let theme_name =
					view.read(cx).config.general.interface.theme.clone();
				crate::data::theme::apply_theme_by_name(
					&theme_name,
					Some(window),
					cx,
				);
				cx.new(|cx| Root::new(view, window, cx))
			},
		)
		.expect("Failed to open GPUI window");
	});
}
