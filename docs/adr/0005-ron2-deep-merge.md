# `ron2` deep-merge for config layering

Config overrides (CLI `--override`) are layered via `deep_merge` operating on `ron2::Value` instead of `ron::Value`. `ron::Value` is lossy for config shape: it collapses named struct fields into unordered maps, tuples into sequences, and bare unit variants into `()`, breaking round-trip. `ron2` preserves the structure, so an override can patch any subtree of the RON config without rewriting the file. Pinned by a test that shows `ron2` preserves shape while `ron` is lossy.
