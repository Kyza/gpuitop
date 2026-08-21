# 08 — Windows window picker

**What to build:** the "click a window to find its process" crosshair tool works on Windows. The window_picker platform path currently reports `false`/`None`. Implement it with Win32 APIs (enumerate/walk windows, pick under cursor, map window → PID) so the picked process pre-fills the search, as it does on Linux.

**Blocked by:** 01, 02.

**Status:** ready-for-agent

- [ ] Crosshair tool lets the user click a window on Windows and selects its process in the table
- [ ] Fallback path when no interactive display session is available
- [ ] The picked window maps to the correct PID

References: `.scratch/windows-dev-env/research.md` §1.1 (window_picker stub), CONTEXT-MAP (picker → search keyword).
