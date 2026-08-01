use crate::model::{Filter, ProcessInfo};
use gpui::*;
use gpui_component::{ActiveTheme, IconName};

pub fn theme_dark_or_light(cx: &App) -> bool {
	cx.theme().is_dark()
}

pub fn state_info(state: char, dark: bool) -> (&'static str, Hsla) {
	match (state, dark) {
		('R', true) => ("Running", hsla(140.0 / 360.0, 0.55, 0.50, 1.0)),
		('R', false) => ("Running", hsla(140.0 / 360.0, 0.50, 0.35, 1.0)),
		('S', true) => ("Sleeping", hsla(0.0 / 360.0, 0.0, 0.55, 1.0)),
		('S', false) => ("Sleeping", hsla(0.0 / 360.0, 0.0, 0.42, 1.0)),
		('D', true) => ("Disk Sleep", hsla(0.0 / 360.0, 0.60, 0.55, 1.0)),
		('D', false) => ("Disk Sleep", hsla(0.0 / 360.0, 0.55, 0.40, 1.0)),
		('Z', true) => ("Zombie", hsla(0.0 / 360.0, 0.55, 0.45, 1.0)),
		('Z', false) => ("Zombie", hsla(0.0 / 360.0, 0.50, 0.32, 1.0)),
		('T', true) => ("Stopped", hsla(40.0 / 360.0, 0.70, 0.50, 1.0)),
		('T', false) => ("Stopped", hsla(40.0 / 360.0, 0.65, 0.37, 1.0)),
		('t', true) => ("Tracing", hsla(40.0 / 360.0, 0.65, 0.55, 1.0)),
		('t', false) => ("Tracing", hsla(40.0 / 360.0, 0.60, 0.42, 1.0)),
		('I', true) => ("Idle", hsla(200.0 / 360.0, 0.40, 0.55, 1.0)),
		('I', false) => ("Idle", hsla(200.0 / 360.0, 0.35, 0.42, 1.0)),
		('X', true) => ("Dead", hsla(0.0 / 360.0, 0.40, 0.40, 1.0)),
		('X', false) => ("Dead", hsla(0.0 / 360.0, 0.35, 0.28, 1.0)),
		_ => (
			"Unknown",
			hsla(0.0 / 360.0, 0.0, if dark { 0.50 } else { 0.40 }, 1.0),
		),
	}
}

pub fn tag_color(tag: TagType, dark: bool) -> Hsla {
	match (tag, dark) {
		(TagType::Gui, true) => hsla(215.0 / 360.0, 0.75, 0.60, 1.0),
		(TagType::Gui, false) => hsla(215.0 / 360.0, 0.65, 0.40, 1.0),
		(TagType::User, true) => hsla(190.0 / 360.0, 0.75, 0.55, 1.0),
		(TagType::User, false) => hsla(190.0 / 360.0, 0.65, 0.35, 1.0),
		(TagType::System, true) => hsla(40.0 / 360.0, 0.80, 0.55, 1.0),
		(TagType::System, false) => hsla(40.0 / 360.0, 0.75, 0.38, 1.0),
		(TagType::Services, true) => hsla(140.0 / 360.0, 0.55, 0.50, 1.0),
		(TagType::Services, false) => hsla(140.0 / 360.0, 0.50, 0.35, 1.0),
		(TagType::Kernel, true) => hsla(0.0 / 360.0, 0.0, 0.55, 1.0),
		(TagType::Kernel, false) => hsla(0.0 / 360.0, 0.0, 0.38, 1.0),
		(TagType::Vram, true) => hsla(270.0 / 360.0, 0.60, 0.60, 1.0),
		(TagType::Vram, false) => hsla(270.0 / 360.0, 0.55, 0.42, 1.0),
		(TagType::Parent, true) => hsla(0.0 / 360.0, 0.0, 0.60, 1.0),
		(TagType::Parent, false) => hsla(0.0 / 360.0, 0.0, 0.45, 1.0),
		(TagType::Electron, true) => hsla(170.0 / 360.0, 0.70, 0.55, 1.0),
		(TagType::Electron, false) => hsla(170.0 / 360.0, 0.60, 0.38, 1.0),
	}
}

#[derive(Clone, Copy)]
pub enum TagType {
	Gui,
	User,
	System,
	Services,
	Kernel,
	Vram,
	Parent,
	Electron,
}

pub fn tag_icons(proc: &ProcessInfo, cx: &App) -> Vec<(IconName, Hsla)> {
	let dark = theme_dark_or_light(cx);
	let mut tags = Vec::new();
	if proc.is_gui {
		tags.push((IconName::LayoutDashboard, tag_color(TagType::Gui, dark)));
	}
	if proc.is_owned_by_current_user && !proc.is_gui {
		tags.push((IconName::User, tag_color(TagType::User, dark)));
	}
	if !proc.is_kthread && !proc.is_owned_by_current_user && proc.ppid != 1 {
		tags.push((IconName::Settings2, tag_color(TagType::System, dark)));
	}
	if proc.ppid == 1 {
		tags.push((
			IconName::SquareTerminal,
			tag_color(TagType::Services, dark),
		));
	}
	if proc.is_kthread {
		tags.push((IconName::Cpu, tag_color(TagType::Kernel, dark)));
	}
	if proc.vram_bytes.is_some() {
		tags.push((IconName::Eye, tag_color(TagType::Vram, dark)));
	}
	if proc.is_electron {
		tags.push((IconName::Globe, tag_color(TagType::Electron, dark)));
	}
	tags
}

pub fn filter_icon(filter: &Filter) -> Option<IconName> {
	match filter {
		Filter::Gui => Some(IconName::LayoutDashboard),
		Filter::User => Some(IconName::User),
		Filter::System => Some(IconName::Settings2),
		Filter::Services => Some(IconName::SquareTerminal),
		Filter::Kernel => Some(IconName::Cpu),
		Filter::Parent => Some(IconName::FolderOpen),
		Filter::Vram => Some(IconName::Eye),
		Filter::Electron => Some(IconName::Globe),
		Filter::ProcessState(_) => Some(IconName::Heart),
		Filter::Pid(_) => None,
	}
}

pub fn filter_color(filter: &Filter, cx: &App) -> Hsla {
	let dark = theme_dark_or_light(cx);
	match filter {
		Filter::Gui => tag_color(TagType::Gui, dark),
		Filter::User => tag_color(TagType::User, dark),
		Filter::System => tag_color(TagType::System, dark),
		Filter::Services => tag_color(TagType::Services, dark),
		Filter::Kernel => tag_color(TagType::Kernel, dark),
		Filter::Vram => tag_color(TagType::Vram, dark),
		Filter::Parent => tag_color(TagType::Parent, dark),
		Filter::Electron => tag_color(TagType::Electron, dark),
		Filter::ProcessState(_) => tag_color(TagType::Gui, dark),
		Filter::Pid(_) => tag_color(TagType::Services, dark),
	}
}
