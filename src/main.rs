mod app;
mod collect;
mod config;
mod model;
mod tabs;
mod widgets;

use gpui::*;
use gpui_component::Root;

#[hotpath::main]
fn main() {
	if std::env::args().any(|a| a == "--profile") {
		std::env::set_var("HOTPATH_OUTPUT_FORMAT", "table");
		eprintln!(
			"[gpuitop] Profiling enabled. Run `hotpath console` in another \
			 terminal for live TUI."
		);
		eprintln!(
			"[gpuitop] Metrics server: http://127.0.0.1:2501 (default)"
		);
	}
	let app = gpui_platform::application()
		.with_assets(gpui_component_assets::Assets);
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
				..Default::default()
			},
			|window, cx| {
				let view = cx.new(|cx| app::App::new(cx));
				cx.new(|cx| Root::new(view, window, cx))
			},
		)
		.unwrap();
	});
}
