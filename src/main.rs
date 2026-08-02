#![recursion_limit = "512"]
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
	let app = gpui_platform::application().with_assets(
		layered::LayeredAssets::new()
			.with(gpui_component_assets::Assets)
			.with(lucide::LucideAssets),
	);
	app.run(move |cx: &mut App| {
		gpui_component::init(cx);
		gpui_component::Theme::change(
			gpui_component::ThemeMode::Dark,
			None,
			cx,
		);
		cx.open_window(
			WindowOptions {
				window_bounds: Some(WindowBounds::Windowed(Bounds {
					origin: point(px(100.0), px(100.0)),
					size: size(px(1100.0), px(700.0)),
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
				let view = cx.new(|cx| app_view::App::new(cx));
				let theme = view.read(cx).theme.get();
				app_view::apply_theme(theme, window, cx);
				cx.new(|cx| Root::new(view, window, cx))
			},
		)
		.unwrap();
	});
}
