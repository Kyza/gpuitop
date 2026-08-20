# Built-in themes unpack to disk; families pair light/dark

Built-in theme JSONs are unpacked to `<config_dir>/gpuitop/themes/` (only if absent) and the directory is watched, so users can edit or add themes without a custom theme loader. Themes are grouped into `ThemeFamily` by name prefix (before the last space); selecting a theme also installs its opposite-mode sibling so dark/light stay consistent.

The alternatives — read-only embedded themes or a bespoke theme-authoring format — either block user customization or duplicate gpui_component's theme machinery. The cost is a filesystem unpack step and a name-prefix heuristic for family grouping.
