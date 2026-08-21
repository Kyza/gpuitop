# 09 — Windows elevation + service detection

**What to build:** the two remaining platform stubs work on Windows. Elevation: the shield button detects whether the app is elevated and can relaunch elevated (e.g. shell "run as administrator" / UAC) instead of `pkexec`. Service manager: init-system detection returns something meaningful or cleanly "unknown" without breaking, and service/child-process classification behaves sanely.

**Blocked by:** 01, 02.

**Status:** ready-for-agent

- [ ] Elevation state is detected correctly on Windows
- [ ] Elevation relaunch path works (or cleanly declines where Windows can't)
- [ ] Service detection/classification returns a sane result on Windows (real where feasible, graceful unknown otherwise)
- [ ] No Unix-only assumptions leak into the Windows path

References: `.scratch/windows-dev-env/research.md` §1.1 (elevation + service_manager stubs).
