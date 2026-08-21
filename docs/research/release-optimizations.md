# Release profile optimization research

Target: `gpuitop` (gpui/gpui-component from git + wgpu + procfs + nucleo). Current
`[profile.release]`: `lto = "thin"`, `strip = "symbols"`.

Primary sources: [Cargo Book — Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)
and [rustc — Codegen Options](https://doc.rust-lang.org/rustc/codegen-options/index.html).

## 1. Option-by-option

| Option | Verdict | Build-time cost | Notes / source |
|---|---|---|---|
| `lto = "thin"` | **keep** | thin ≪ fat | Thin is "similar" to fat gains at "substantially less time"; for large graphs thin can beat fat (`rustc/codegen-options#lto`, `cargo/reference/profiles#lto`). Fat only worth one experimental A/B. |
| `lto = "fat"` | optional test | large | Whole-graph IPO; marginal gain over thin on a wgpu-sized graph. |
| `opt-level = 3` | **keep**, try `"s"` | — | `"z"` is "optimize for size, but more aggressively. Often results in larger binaries than `s`" (`rustc/codegen-options#opt-level`). `"s"`/`"z"` "not necessarily smaller" (`cargo/reference/profiles#opt-level`). For an idle, event-driven UI, `"s"` usually costs little runtime speed. Measure both. |
| `codegen-units = 1` | **recommended** | slower compile (loses per-crate parallelism) | "1 may improve performance of generated code, but may be slower to compile" (`rustc/codegen-options#codegen-units`). Release default is 16 (`cargo/reference/profiles#codegen-units`). Release is not incremental, so this only affects full rebuild time. |
| `panic = "abort"` | **safe here — see §2** | small win (shorter link) | Removes landing pads + unwind tables → smaller `.text`, marginal speed gain. |
| `strip = "symbols"` | already set | — | `strip = true` ≡ `strip = "symbols"` (`cargo/reference/profiles#strip`). Nothing to gain. `"symbols"` can cripple backtraces (`rustc/codegen-options#strip`) — acceptable for a GUI, not a CLI. |
| `overflow-checks` | default ok | — | Already `false` in release (`cargo/reference/profiles#overflow-checks`). |
| `debug-assertions` | default ok | — | Already `false` in release (`cargo/reference/profiles#debug-assertions`). |

## 2. Is `panic = "abort"` safe? — YES

`catch_unwind`/`resume_unwind` in the **pinned** revisions actually linked:

- **gpui** = `zed#5e1fd39` (root `Cargo.lock`, `git+https://github.com/zed-industries/zed#5e1fd392f…`).
  All hits are excluded from the release build:
  - `crates/gpui/src/test.rs` — gated by `#[cfg(any(test, feature = "test-support"))]`
    (`crates/gpui/src/gpui.rs:56-57`); `test-support` is **not** enabled anywhere in `Cargo.lock`.
  - `crates/gpui_windows/src/platform.rs:1475` — Windows-only, inside `cfg!(debug_assertions)`
    (false in release), and its `Err` arm calls `std::process::abort()` anyway.
  - `crates/scheduler/src/{test_scheduler.rs,tests.rs}`, `crates/zed/src/visual_test_runner.rs`,
    `crates/remote_server/src/server.rs` — zed-app/test crates, never linked into gpuitop.
- **gpui-component** = `longbridge#88f102d` (`Cargo.lock`). **Zero** `catch_unwind`/`resume_unwind`
  in the whole tree (the one hit was in a non-pinned checkout rev `e5b8a3f`).
- **Workspace** (`crates/`): zero `catch_unwind`/`resume_unwind` anywhere.

`panic` cannot be per-package overridden (`cargo/reference/profiles#overrides`: "Overrides cannot
specify the `panic`… settings"), so it applies graph-wide — safe here.

**Behavior caveat (not a safety issue):** with `abort`, a panic in the background collector
thread (`crates/gpuitop/src/app_view.rs:114`) now kills the whole process instead of just that
thread. Nothing catches it today either way; abort just makes it visible.

## 3. Workspace / app-specific findings

- **Dominant size driver is gpui → wgpu.** `wgpu` is in `Cargo.lock`, pulled in by
  `gpui_platform` with `runtime_shaders`, `wayland`, `x11`, `font-kit` features
  (root `Cargo.toml:21-26`). Runtime shader compilation brings in naga. Not shrinkable from
  profiles; dropping `runtime_shaders` is a feature decision, not a build one.
- **`rust-embed`** (builtin themes) embeds assets into the binary — adds size by design.
- **`build.rs`** (`crates/gpuitop/build.rs`) only emits a const `DIRECT_DEPS` table for the About
  page + `.desktop` under the `packaging` feature — negligible, no release impact.
- **`hotpath`** features are opt-in; default release builds have none enabled — no overhead.
- No `.cargo/config.toml` and no `RUSTFLAGS` in the workspace. On `x86_64-unknown-linux-gnu`,
  rustc already tries lld/self-contained linking by default (`rustc/codegen-options#linker-features`).
- Measure, don't guess: `cargo bloat --release --crates` finds the real bloat;
  `cargo bloat --release -n 10` shows top symbols. Then A/B `opt-level = "s"` vs `3` and
  `lto = "thin"` vs `"fat"`.

## 4. Recommended config

Balanced (smaller + faster, moderate build-time hit):

```toml
[profile.release]
lto = "thin"        # keep — thin ≈ fat gains, far less link time
codegen-units = 1   # smaller + faster code; costs full-rebuild time only
panic = "abort"     # safe (no catch_unwind in graph); shrinks binary
strip = true        # same as "symbols"; keep
```

Max-size variant (only if the size win matters more than peak throughput — re-benchmark):

```toml
[profile.release]
lto = "thin"
codegen-units = 1
panic = "abort"
strip = true
opt-level = "s"     # prefer "s" over "z"; "z" is often *larger* (rustc doc)
```
