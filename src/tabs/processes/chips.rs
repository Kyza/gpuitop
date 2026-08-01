use crate::model::*;
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	menu::PopupMenuItem,
	tag::Tag,
	Icon, IconName,
};
use std::cell::RefCell;
use std::rc::Rc;
use super::state::ViewState;
use super::ProcessesTab;
use super::theme::{filter_icon, filter_color};

pub fn render_filter_chip(
	filter: &Filter,
	label: &str,
	is_active: bool,
	is_type: bool,
	cx: &mut Context<ProcessesTab>,
) -> AnyElement {
	if is_active && is_type {
		let f = filter.clone();
		let icon = filter_icon(filter);
		let icon_color = filter_color(filter, cx);
		let label_str = label.to_string();
		div()
			.id(ElementId::Name(format!("filt-{label}").into()))
			.cursor(CursorStyle::PointingHand)
			.on_click(cx.listener(move |this, _, _, cx| {
				this.toggle_filter(f.clone(), cx);
			}))
			.child(
				Tag::info().child(
					div()
						.flex()
						.flex_row()
						.items_center()
						.gap(px(4.0))
						.when(icon.is_some(), |el| {
							el.child(
								Icon::new(icon.unwrap())
									.size(px(12.0))
									.text_color(icon_color),
							)
						})
						.child(label_str),
				),
			)
			.into_any_element()
	} else if is_active {
		// PidFilter active: shows ✕ to remove
		let f = filter.clone();
		Tag::success()
			.child(
				div()
					.flex()
					.flex_row()
					.items_center()
					.gap(px(4.0))
					.child(label.to_string())
					.child(
						div()
							.id(ElementId::Name(
								format!("rm-filt-{label}").into(),
							))
							.cursor(CursorStyle::PointingHand)
							.on_click(cx.listener(move |this, _, _, cx| {
								ViewState::mutate(&this.view_state, |s| {
									s.filters.retain(|flt| flt != &f);
								});
								cx.notify();
							}))
							.child(Icon::new(IconName::Close).size(px(10.0))),
					),
			)
			.into_any_element()
	} else {
		let f = filter.clone();
		let icon = filter_icon(filter);
		let icon_color = filter_color(filter, cx);
		let label_str = label.to_string();
		div()
			.id(ElementId::Name(format!("filt-{label}").into()))
			.cursor(CursorStyle::PointingHand)
			.on_click(cx.listener(move |this, _, _, cx| {
				this.toggle_filter(f.clone(), cx);
			}))
			.child(
				Tag::new().child(
					div()
						.flex()
						.flex_row()
						.items_center()
						.gap(px(4.0))
						.when(icon.is_some(), |el| {
							el.child(
								Icon::new(icon.unwrap())
									.size(px(12.0))
									.text_color(icon_color),
							)
						})
						.child(label_str),
				),
			)
			.into_any_element()
	}
}

pub fn get_state_item(
	state: char,
	active: &[char],
	view_state: Rc<RefCell<ViewState>>,
) -> PopupMenuItem {
	let label = crate::model::state_label(state).to_string();
	let checked = active.contains(&state);
	PopupMenuItem::Item {
		icon: None,
		label: label.into(),
		disabled: false,
		checked,
		is_link: false,
		action: None,
		handler: Some(std::rc::Rc::new(move |_, _, _| {
			ViewState::mutate(&view_state, |s| {
				if let Some(pos) = s
					.filters
					.iter()
					.position(|f| *f == Filter::ProcessState(state))
				{
					s.filters.remove(pos);
				} else {
					s.filters.push(Filter::ProcessState(state));
				}
			});
		})),
	}
}
