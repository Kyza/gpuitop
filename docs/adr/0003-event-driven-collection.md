# Event-driven collection: background thread + wake channel

A background thread owns the `CollectorState` and loops `collect_snapshot(&mut state)` at the refresh interval. Each snapshot is sent over an `mpsc` channel; a wake signal over an `async_channel` triggers `cx.notify()` — so rendering is data-driven with no perpetual `on_next_frame` loop. UI never reads `/proc` directly.

The trade-off: a second copy of state and a wake-signal mechanism, in exchange for never blocking the render thread on slow filesystem reads. The properties window reuses the same pattern with its own wake channel.
