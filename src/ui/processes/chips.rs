use crate::data::model::*;
use crate::data::state::ViewState;
use gpui_component::menu::PopupMenuItem;
use std::cell::RefCell;
use std::rc::Rc;

pub fn get_state_item(
	state: char,
	active: &[char],
	view_state: Rc<RefCell<ViewState>>,
) -> PopupMenuItem {
	let label = crate::data::model::state_label(state).to_string();
	let checked = active.contains(&state);
	PopupMenuItem::Item {
		icon: None,
		label: label.into(),
		disabled: false,
		checked,
		is_link: false,
		action: None,
		handler: Some(Rc::new(move |_, _, _| {
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
