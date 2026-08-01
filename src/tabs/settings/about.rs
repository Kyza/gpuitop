use gpui::prelude::*;
use gpui::*;
use gpui_component::{
	setting::{SettingGroup, SettingItem, SettingPage},
	ActiveTheme, Icon, IconName,
};

pub fn about_page() -> SettingPage {
	SettingPage::new("About")
		.default_open(false)
		.icon(Icon::new(IconName::Info))
		.group(
			SettingGroup::new().item(SettingItem::render(|_options, _, cx| {
				gpui_component::v_flex()
					.gap_3()
					.w_full()
					.items_center()
					.justify_center()
					.child(
						Icon::new(IconName::Cpu)
							.size(px(32.0))
							.text_color(cx.theme().muted_foreground),
					)
					.child(
						gpui_component::label::Label::new("gpuitop v0.1.0")
							.text_lg(),
					)
					.child(
						gpui_component::label::Label::new(
							"Linux-first process manager",
						)
						.text_sm()
						.text_color(cx.theme().muted_foreground),
					)
					.child(
						gpui_component::label::Label::new(
							"Built with GPUI & gpui-component",
						)
						.text_sm()
						.text_color(cx.theme().muted_foreground),
					)
					.into_any()
			})),
		)
}
