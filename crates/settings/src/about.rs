use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	description_list::{DescriptionItem, DescriptionList},
	setting::{SettingGroup, SettingItem, SettingPage},
	ActiveTheme, Icon, IconName,
};
use gpuitop_components::assets::lucide::LucideIcon;
use gpuitop_core::about::DepInfo;

fn md(src: impl Into<SharedString>, id: &str) -> AnyElement {
	gpui_component::text::TextView::markdown(
		SharedString::from(id.to_string()),
		src,
	)
	.into_any_element()
}

fn dep_list(
	name: &str,
	version: &str,
	license: &str,
	description: &str,
	website: &str,
	repository: &str,
	id_prefix: &str,
) -> DescriptionList {
	let mut list = DescriptionList::new()
		.columns(2)
		.child(DescriptionItem::new(name).value(version))
		.child(DescriptionItem::new("License").value(license));
	if !description.is_empty() {
		list = list.child(
			DescriptionItem::new("Description")
				.value(description)
				.span(2),
		);
	}
	if !website.is_empty() && website != repository {
		list = list.child(
			DescriptionItem::new("Website")
				.value(md(
					format!("[{}]({})", website, website),
					&format!("{}-website", id_prefix),
				))
				.span(2),
		);
	}
	if !repository.is_empty() {
		list = list.child(
			DescriptionItem::new("Repository")
				.value(md(
					format!("[{}]({})", repository, repository),
					&format!("{}-repo", id_prefix),
				))
				.span(2),
		);
	}
	list
}

pub fn about_page(deps: &'static [DepInfo]) -> SettingPage {
	SettingPage::new("About")
		.default_open(false)
		.icon(Icon::new(IconName::Info))
		.group(SettingGroup::new().item(SettingItem::render(
			|_options, _, cx| {
				let mut entries: Vec<(&str, DescriptionList)> = Vec::new();

				entries.push({
					let l = DescriptionList::new()
						.columns(2)
						.child(
							DescriptionItem::new("gpuitop")
								.value(env!("CARGO_PKG_VERSION")),
						)
						.child(
							DescriptionItem::new("License")
								.value(env!("CARGO_PKG_LICENSE")),
						)
						.child(
							DescriptionItem::new("Author")
								.value(env!("CARGO_PKG_AUTHORS")),
						)
						.child(
							DescriptionItem::new("Repository")
								.value(md(
									"https://github.com/Kyza/gpuitop",
									"gpuitop-repo",
								))
								.span(2),
						);
					("gpuitop", l)
				});

				for dep in deps.iter() {
					entries.push((
						dep.name,
						dep_list(
							dep.name,
							dep.version,
							dep.license,
							dep.description,
							dep.homepage,
							dep.repository,
							dep.name,
						),
					));
				}

				entries.push((
					"lucide",
					DescriptionList::new()
						.columns(2)
						.child(
							DescriptionItem::new("lucide")
								.value("Beautiful & consistent icon set"),
						)
						.child(DescriptionItem::new("License").value("ISC"))
						.child(
							DescriptionItem::new("Website")
								.value(md(
									"https://lucide.dev",
									"lucide-website",
								))
								.span(2),
						)
						.child(
							DescriptionItem::new("Repository")
								.value(md(
									"https://github.com/lucide-icons/lucide",
									"lucide-repo",
								))
								.span(2),
						),
				));

				entries.sort_by_key(|(name, _)| name.to_lowercase());
				let mut lists_with_seps = Vec::new();
				for (i, (_, list)) in entries.into_iter().enumerate() {
					if i > 0 {
						lists_with_seps.push(
							div()
								.h_px()
								.w_full()
								.border_t_1()
								.border_color(cx.theme().border)
								.into_any_element(),
						);
					}
					lists_with_seps.push(list.into_any_element());
				}

				gpui_component::v_flex()
					.gap_3()
					.w_full()
					.items_center()
					.justify_center()
					.child(
						LucideIcon::Info
							.icon()
							.size(px(32.0))
							.text_color(cx.theme().muted_foreground),
					)
					.child(
						gpui_component::label::Label::new(format!(
							"gpuitop v{}",
							env!("CARGO_PKG_VERSION"),
						))
						.text_lg(),
					)
					.child(
						gpui_component::label::Label::new(env!(
							"CARGO_PKG_DESCRIPTION"
						))
						.text_sm()
						.text_color(cx.theme().muted_foreground),
					)
					.child(
						div().w_full().pt_4().child(
							gpui_component::v_flex()
								.gap(px(12.))
								.children(lists_with_seps),
						),
					)
					.into_any()
			},
		)))
}
