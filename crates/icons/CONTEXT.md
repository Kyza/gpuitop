# Icons

Resolves process icons from Linux `.desktop` files. The cache is built once at startup and shared (as `Arc`) into the collector thread.

## Language

**DesktopEntry**:
A parsed `.desktop` file: `name`, `icon_name`, `exec` — the three fields a process icon needs.
_Avoid_: ".desktop file" when the parsed value is meant

**DesktopEntryCache**:
A map from lowercase exec-basename to `DesktopEntry`. Lookup is two-tier: exact exe-basename hit first, then a substring scan of the cmdline — the fallback that catches Electron-style helper processes.
_Avoid_: "icon cache" (the cache stores entries, not resolved icon paths)

**load_cache**:
Scans the desktop-file directories (`/usr/share/applications`, `~/.local/share/applications`) and returns a populated `DesktopEntryCache`. Called once at startup.

**icon_name**:
The icon key carried on `ProcessSnapshot`, resolved to a file path at render time.
**resolve_icon_path**:
Turns an `icon_name` into a path via the hicolor size ladder (`[256…16]` × `[png, svg, xpm]`), then `/usr/share/pixmaps`.
_Avoid_: "icon path ladder" (implementation detail)

**Minimal .desktop parser**:
Hand-rolled parser reading only the `[Desktop Entry]` section's first `Name=`/`Icon=`/`Exec=`. No parsing crate; no localized or group keys.

## Relationships

- **Icons → Snapshot**: the `Arc<DesktopEntryCache>` lives in `CollectorState`; the collector stamps `ProcessSnapshot.icon_name`.
- **Icons → Processes**: `resolve_icon_path` runs per-row at render time.
