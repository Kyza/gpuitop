# Components

Shared UI building blocks used across panels: selectable verbatim text, process-tag colors/icons, theme loading, and the icon asset sources.

## Language

**SelectableText**:
A label widget that renders text verbatim yet stays selectable/copyable. gpui has no built-in selectable text, and gpui_component's `TextView` only parses Markdown/HTML — so `SelectableText` HTML-escapes the input and wraps `TextView::html`: display exact, copied selection original. Each instance needs a stable unique id.
_Avoid_: "copyable text" (selection matters too)

**TagType**:
The process classification tags shown in the table: `Gui`, `User`, `System`, `Services`, `Kernel`, `Vram`, `Nvidia`, `Amd`, `Parent`, `Electron` — the UI twin of core's `Filter`.
_Avoid_: "filter type" (that's the core enum; tags are the visual layer)

**Tag icons**:
The icon + color + tooltip row computed per process from its classification flags (`is_gui`, `is_kthread`, `is_owned_by_current_user`, `is_electron`, VRAM > 0, service-ness). The classification logic lives here, not in core.
**State color**:
Maps a process state char to its human label (core's `state_label`) plus a dark/light-aware color.

**Theme** (tag colors):
The `theme.rs` module: tag/state/filter color semantics. One of four things called "theme".
**Theme** (loading):
The `themes.rs` module: built-in theme loading, `ThemeFamily`, and application. Distinct from the tag-color module and from gpui_component's `Theme`.
**ThemeFamily**:
A named group of theme variants (name + mode pairs), e.g. "Default" with Dark/Light. Selecting one installs its opposite-mode sibling too.
_Avoid_: "theme group" (family is the established name)

**Theme menu**:
The shared theme picker (`build_theme_menu`) used by both the titlebar and the Settings General page, with hover preview.
_Avoid_: duplicating it — one component, two call sites

**LucideIcon**:
The ~40 named icons used across the app; each maps to an SVG filename from the vendored `lucide/` submodule.
**LayeredAssets**:
A composite `AssetSource` stacking multiple sources; later layers win.

## Relationships

- **Components → Core**: `TagType` mirrors `Filter`; `filter_color`/`filter_icon` pair core filters with tag colors.
- **Components → gpuitop/settings**: both host the theme menu; both apply themes via `apply_theme_by_name`.
- **Theme dir**: built-in themes are unpacked to `<config_dir>/gpuitop/themes/` so users can edit or add them.
