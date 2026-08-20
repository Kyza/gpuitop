# `build.rs` emits `built.rs` for the About dependency list

`build.rs` runs `cargo_metadata` at build time and generates `built.rs` containing `DIRECT_DEPS: &[gpuitop_core::about::DepInfo]` — the bin's direct runtime deps, filtered to those with a license or repository, sorted and serialized as Rust literals. `main.rs` `include!`s it and passes it into `SettingsTab::new`.

The `DepInfo` type lives in core so both the generator and the renderer share one definition. The alternative — hardcoding a dependency list in the About page — drifts the moment a dependency changes; deriving it at build time keeps the list accurate with zero runtime cost.
