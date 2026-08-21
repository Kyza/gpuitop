# 05 — Windows properties window

**What to build:** opening a process's Properties window shows real data on Windows. The properties platform path currently returns `Err(ReadError::Dead)` and the window body renders "not available on this platform". Implement real per-process property collection (paths, memory, command line, etc. via Windows APIs) and wire the window to it.

**Blocked by:** 01, 02.

**Status:** ready-for-agent

- [ ] Properties window opens for a Windows process and shows real values
- [ ] Fields that Windows can't provide show a graceful "unavailable", not an error state
- [ ] `users` (Unix-only) stays out of the Windows build path

References: `.scratch/windows-dev-env/research.md` §1.1 (properties + properties/window stubs).
