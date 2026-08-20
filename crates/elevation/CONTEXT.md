# Elevation

Relaunches the app as root via `pkexec` so system processes can be managed. Backed by the titlebar shield button.

## Language

**is_elevated**:
Whether the current instance runs as root (`geteuid() == 0`); false on Windows. Drives the shield button's tooltip/state.

**relaunch_elevated**:
Re-executes the current executable with the same args and cwd **via `pkexec`**, spawning a child and returning immediately; the caller then quits, expecting the pkexec prompt and the root instance to take over. The parent never waits on the child — a spawn-and-quit handoff.
_Avoid_: "re-exec" (that implies `exec`-style replacement; this spawns)

**env whitelist**:
The display/D-Bus environment forwarded to the pkexec child (`DISPLAY`, `XAUTHORITY`, `WAYLAND_DISPLAY`, `XDG_RUNTIME_DIR`, `XDG_SESSION_TYPE`, `XDG_CURRENT_DESKTOP`, `XDG_SESSION_DESKTOP`, `DBUS_SESSION_BUS_ADDRESS`, `LANG`) — the plumbing pkexec would otherwise strip, and what lets the elevated instance render and reach the display server.

## Relationships

- **Elevation → gpuitop**: the App calls `relaunch_elevated()` from the shield button and quits on success; errors surface in the status bar.
- **Linux-only**: macOS/Windows return explicit "not supported" errors — a deliberate carve-out (see ADR-0007).
