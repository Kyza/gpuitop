# 07 — Windows icons

**What to build:** processes show their real icons on Windows. The icons platform path currently returns an empty cache and no icon path. Implement icon resolution for Windows executables (e.g. extract from the .exe / registered app icon) so the process table displays them, replacing the Linux `.desktop`-based resolution.

**Blocked by:** 01, 02.

**Status:** ready-for-agent

- [ ] Process rows display real icons in the Windows build
- [ ] Missing/unknown icons fall back gracefully (no broken render)
- [ ] Icon lookup is cheap enough for the table refresh cadence

References: `.scratch/windows-dev-env/research.md` §1.1 (icons stub).
