# 04 — Windows process signals + affinity

**What to build:** right-clicking a process offers working terminate/stop/continue actions and a CPU-affinity picker on Windows. The context menu currently calls `libc::kill`/SIG* unconditionally; replace that with a real Windows mechanism (e.g. OpenProcess + TerminateProcess / SuspendThread, or a signals-via-API equivalent) and implement the affinity stubs via Windows affinity APIs.

**Blocked by:** 01, 02.

**Status:** ready-for-agent

- [ ] Terminate/stop/continue from the process context menu affects a real Windows process
- [ ] Affinity picker reads and sets a process's allowed CPUs on Windows
- [ ] No Unix-only symbols leak into the Windows build path

References: `.scratch/windows-dev-env/research.md` §1.2 (current `libc::kill` blocker), §1.1 (affinity stub).
