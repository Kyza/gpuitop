# Lazy cache invalidation: set-to-None + generation counter

Derived data (`cum_cache`, `pid_index`, `descendant_counts`) is stored as `Rc<RefCell<Option<...>>>` and rebuilt lazily on first use. Invalidation is wholesale: `set_snapshot` resets each to `None`. `ViewState` carries a wrapping `generation` counter, bumped on every mutation via `ViewState::mutate`; the filtered-rows and tree caches key on `(snapshot timestamp, generation)` and self-invalidate.

The trade-off: per-snapshot caches are rebuilt even when only a slice changed, and every view mutation forces a pipeline recompute. In exchange there is exactly one invalidation path (no fine-grained dirty tracking), which keeps the hot rendering path simple and correct. Memoized indexes that survive a snapshot (e.g. the `is_gui` flag cache) are guarded separately by `starttime` against PID reuse.
