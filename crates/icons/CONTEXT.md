# Icons

Resolves process icons from Linux `.desktop` files in two stages: (1) match a
running process's cmdline to a desktop entry (`.desktop` → `icon_name`), done
once at startup against the desktop-file cache and stamped per process; (2)
resolve the `icon_name` to an on-disk icon path via linicon (XDG icon theme
spec), memoized in an async-warmed path cache.

## Language

**DesktopEntry**:
A parsed `.desktop` file: `name`, `icon_name`, `exec` — the three fields a process icon needs.
_Avoid_: ".desktop file" when the parsed value is meant

**DesktopEntryCache**:
A set of desktop entries indexed by multiple lowercase keys: exec-basename (`by_exec`), plus every exec-basename, flatpak app-id, and normalized desktop Name as tokens (`by_token`), plus normalized Names for prefix matching (`by_name`). Lookup is three-tier: exact exe-basename hit first, then a scan of the cmdline's **path components** and dotted app-ids only (catches Electron helpers via the app path `/usr/lib/vesktop/app.asar`; never matches bare flag args like `--search vesktop`), then exe-basename-prefix-of-Name (catches `kate-server` → Kate). Longest key wins; user-dir entries win ties.
_Avoid_: "icon cache" (the cache stores entries, not resolved icon paths)

**Exec unwrapping**:
`extract_exec_basename` resolves the real command a desktop entry launches, skipping wrapper noise: `env VAR=... cmd` yields `cmd`, `flatpak run --command=CMD` yields `CMD`. `sh -c`/`bash -c` wrappers are skipped entirely — the quoted command is not parsed and `sh`/`bash` are too short to be safe index keys.

**load_cache**:
Scans the desktop-file directories (`/usr/share/applications`, `~/.local/share/applications`) and returns a populated `DesktopEntryCache`. Called once at startup.

**icon_name**:
The icon key carried on `ProcessSnapshot`, resolved to a file path at render time.
**resolve_icon_path**:
Cache-only: reads the memoized path for an `icon_name` and returns `None` until the cache is warmed. Never performs a linicon lookup on the render thread.
**warm_icon_paths**:
Async warmup, called from the collector thread after each tick with the seen icon names. Resolves new names via linicon (XDG theme-aware: current theme → fallbacks → hicolor). Re-checks the system icon theme at most once per second; a theme change drops the whole path cache.
_Avoid_: "icon path ladder" (the old hicolor-only `{size}x{size}/apps` scan; gone)

**Minimal .desktop parser**:
Hand-rolled parser reading only the `[Desktop Entry]` section's first `Name=`/`Icon=`/`Exec=`. No parsing crate; no localized or group keys.

## Relationships

- **Icons → Snapshot**: the `Arc<DesktopEntryCache>` lives in `CollectorState`; the collector stamps `ProcessSnapshot.icon_name` and feeds `warm_icon_paths` each tick.
- **Icons → Processes**: `resolve_icon_path` runs per-row at render time (cache read).
