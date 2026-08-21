# 01 — Windows compile gate

**What to build:** the entire workspace compiles for `x86_64-pc-windows-msvc`. Today it cannot: the `users` crate is a non-conditional dependency of `snapshot`/`properties` but is Unix-only, and the process context menu calls `libc::kill`/SIG* unconditionally. Gate the `users` dep to unix (it's only used in Linux proc code) and gate/replace the signal calls on Windows. Optionally add a cargo-xwin cross-compile job to CI so it stays green. Nothing needs to *run* — this ticket only makes the Windows target build.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] `cargo check --target x86_64-pc-windows-msvc` succeeds for the whole workspace (no procfs/users/libc-symbol errors)
- [ ] The Windows stubs still compile (no Windows feature regressions)
- [ ] Optional: CI job running the cross-compile gate

References: `.scratch/windows-dev-env/research.md` §1.2 (two blockers), §2.
