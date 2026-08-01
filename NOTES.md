# Progress Bar Issues — Notes

## Current state
Two 3px progress bars at the bottom of `app.rs`:

1. **Collecting pulse** (yellow): `w(relative(if is_collecting { 1.0 } else { 0.0 }))`
   - `collecting` is an `Arc<AtomicBool>` toggled by the bg thread
   - Shows full-width yellow bar while `collector.tick()` runs

2. **Refresh countdown** (blue): `w(relative(refresh_progress))`
   - `refresh_progress = elapsed / interval` fills 0→1 from `snapshot.timestamp`
   - Should snap back to 0 when collector thread writes new `timestamp`

## Problems

### 1. Countdown doesn't reset
`snapshot.timestamp` is updated by the collector, but GPUI needs `cx.notify()` to re-render.

**Fix attempt**: `cx.on_next_frame(window, |_, _, cx| cx.notify())` in render — schedules a re-render every frame. This should make the bar animate and reset on timestamp change.

### 2. Bar direction
Was `1.0 - refresh_progress` (depleting). Changed to `refresh_progress` (filling). User confirmed it was "decreasing" — now it increases.

### 3. Collector speed
Benchmark: **76ms/tick** for 427 processes (178µs/proc). With 1.5s refresh, that's 5% overhead — acceptable. But the collecting bar shows yellow for ~76ms which flickers noticeably.

**Potential optimizations (not yet done)**:
- Skip `/proc/<pid>/io` reads (not needed for the core table)
- Cache username lookups with `HashMap<u32, String>`
- Parallel process reads with `rayon`
- Batch stat reads (read `/proc/<pid>/stat` in a thread pool)

### 4. `on_next_frame` infinite loop risk
`cx.on_next_frame(window, |_, _, cx| cx.notify())` creates a self-perpetuating render loop. This is intentional for system monitors but consumes GPU resources. Alternatives:
- Use `cx.spawn` with a timer (async closure pattern not yet working due to lifetime issues)
- Use `request_animation_frame` directly
- A production app would use `cx.spawn` with periodic notify every 200ms
