use gpui::{ElementId, IntoElement, SharedString, StyleRefinement, Styled};
use gpui_component::text::TextView;

fn escape_html(s: &str) -> String {
	let mut out = String::with_capacity(s.len());
	for c in s.chars() {
		match c {
			'&' => out.push_str("&amp;"),
			'<' => out.push_str("&lt;"),
			'>' => out.push_str("&gt;"),
			_ => out.push(c),
		}
	}
	out
}

/// A label that renders its text verbatim but is still selectable and
/// copyable (window-level selection, Ctrl/Cmd+C).
///
/// gpui-component's [`TextView`] only parses Markdown/HTML, so it mangles
/// values containing markup characters. This wraps `TextView::html` after
/// HTML-escaping the input, so the text displays exactly as given while the
/// copied selection is the original, unescaped string.
#[derive(Clone)]
pub struct SelectableText {
	inner: TextView,
}

impl SelectableText {
	pub fn new(
		id: impl Into<ElementId>,
		text: impl Into<SharedString>,
	) -> Self {
		let escaped = escape_html(&text.into());
		Self {
			inner: TextView::html(id, escaped).selectable(true),
		}
	}
}

impl Styled for SelectableText {
	fn style(&mut self) -> &mut StyleRefinement {
		<TextView as Styled>::style(&mut self.inner)
	}
}

impl IntoElement for SelectableText {
	type Element = TextView;

	fn into_element(self) -> Self::Element {
		self.inner
	}
}
