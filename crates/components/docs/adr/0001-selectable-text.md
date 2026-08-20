# SelectableText = HTML-escape + `TextView::html`

gpui has no built-in selectable text, and gpui_component's `TextView` only parses Markdown/HTML — which would mangle any value containing markup. `SelectableText` escapes `&`, `<`, `>` and wraps `TextView::html`, so the displayed text is verbatim while the copied selection stays the original string. Each instance needs a stable unique id.

The alternative — a hand-rolled selection layer on gpui's text stack — is a large amount of platform selection logic for a small win. The escaping wrapper inverts a parser's behavior with ~10 lines instead.
