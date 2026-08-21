# 03 — Windows process collection

**What to build:** the Processes tab shows live Windows processes with real CPU and memory, instead of the current `SystemSnapshot::empty()`. Implement real process enumeration in the Windows platform path (e.g. Toolhelp32 / NtQuerySystemInformation), producing the existing `SystemSnapshot` model. GPU-per-process and icon fields can remain unpopulated until tickets 06/07.

**Blocked by:** 01, 02.

**Status:** ready-for-agent

- [ ] Processes tab lists real Windows processes with PID, name, real CPU and memory
- [ ] Sorting and filtering work against the collected data
- [ ] Collection runs on the background-thread collector, same as Linux (no UI reads in the collection path)

References: `.scratch/windows-dev-env/research.md` §1.1 (stub), §3.4 (VM sizing for build/test).
