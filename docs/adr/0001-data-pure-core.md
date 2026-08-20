# Data-pure core

Data crates (`core`, `gpu`, `icons`, `snapshot`, `window_picker`) never import `gpui` or `gpui_component`. The domain model, filtering/sorting engine, and config live in pure Rust so they stay testable and UI-framework-independent. Panel/component crates are the only ones allowed to depend on gpui.

The alternative — a UI-native data model shared app-wide — couples the core domain to GPUI's lifetime and render models. The cost of this decision is the orphan-rule newtype dance where UI traits (e.g. `TableDelegate`) must wrap core types (see processes context ADR).
