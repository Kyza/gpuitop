use std::cell::RefCell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::App as GpuiApp;
use gpui::*;
use gpui_component::menu::{PopupMenu, PopupMenuItem};
use gpui_component::ThemeMode;

use crate::assets::lucide::LucideIcon;
use crate::themes::{apply_theme_by_name, list_theme_families};

fn theme_preview_item(
	menu: PopupMenu,
	name: &SharedString,
	mode: ThemeMode,
	current: &SharedString,
	preview: &Rc<RefCell<Option<SharedString>>>,
	on_commit: &Rc<dyn Fn(&SharedString, &mut GpuiApp)>,
) -> PopupMenu {
	let name_owned = name.clone();
	let item_name = name_owned.clone();
	let item_current = name_owned == *current;
	let preview_name = name_owned.clone();
	let hover_preview = preview.clone();
	let click_preview = preview.clone();
	let on_commit = on_commit.clone();

	let item = PopupMenuItem::element({
		let name_display = name_owned.clone();
		move |_window: &mut Window, _cx: &mut GpuiApp| {
			let hover_preview = hover_preview.clone();
			let preview_name = preview_name.clone();
			div()
				.id(ElementId::Name(format!("theme-{name_display}").into()))
				.w_full()
				.flex()
				.flex_row()
				.items_center()
				.gap(px(4.0))
				.child(if mode.is_dark() {
					LucideIcon::Moon.icon().size(px(12.0)).text_color(
						gpui_component::theme::Theme::global(_cx)
							.muted_foreground,
					)
				} else {
					LucideIcon::Sun.icon().size(px(12.0)).text_color(
						gpui_component::theme::Theme::global(_cx)
							.muted_foreground,
					)
				})
				.child(SharedString::from(name_display.as_ref()))
				.child(div().flex_grow(1.0))
				.on_hover(
					move |hovered: &bool,
					      window: &mut Window,
					      cx: &mut GpuiApp| {
						if *hovered {
							if hover_preview.borrow().is_none() {
								let original =
									gpui_component::theme::Theme::global(cx)
										.theme_name()
										.clone();
								*hover_preview.borrow_mut() = Some(original);
							}
							apply_theme_by_name(
								&preview_name,
								Some(window),
								cx,
							);
						} else {
							let original = hover_preview.borrow().clone();
							if let Some(original) = original {
								apply_theme_by_name(
									&original,
									Some(window),
									cx,
								);
								*hover_preview.borrow_mut() = None;
							}
						}
					},
				)
		}
	})
	.checked(item_current)
	.disabled(item_current)
	.on_click(
		move |_: &ClickEvent, window: &mut Window, cx: &mut GpuiApp| {
			*click_preview.borrow_mut() = None;
			(on_commit)(&item_name, cx);
			apply_theme_by_name(&item_name, Some(window), cx);
		},
	);
	menu.item(item)
}

pub fn build_theme_menu(
	mut menu: PopupMenu,
	on_commit: &Rc<dyn Fn(&SharedString, &mut GpuiApp)>,
	window: &mut Window,
	cx: &mut Context<PopupMenu>,
) -> PopupMenu {
	let families = list_theme_families(cx);
	let current = gpui_component::theme::Theme::global(cx)
		.theme_name()
		.clone();
	let preview = Rc::new(RefCell::new(None::<SharedString>));

	for family in families {
		if family.variants.len() == 1 {
			let (name, mode) = &family.variants[0];
			menu = theme_preview_item(
				menu, name, *mode, &current, &preview, on_commit,
			);
		} else {
			let family_name = SharedString::from(family.name.as_ref());
			let family_has_current =
				family.variants.iter().any(|(n, _)| n == &current);
			let submenu = PopupMenu::build(window, cx, {
				let variants = family.variants.clone();
				let current = current.clone();
				let preview = preview.clone();
				let on_commit = on_commit.clone();
				move |mut menu, _, _| {
					for (name, mode) in &variants {
						menu = theme_preview_item(
							menu, name, *mode, &current, &preview, &on_commit,
						);
					}
					menu
				}
			});
			menu = menu.item(
				PopupMenuItem::submenu(family_name, submenu)
					.checked(family_has_current),
			);
		}
	}

	menu
}
