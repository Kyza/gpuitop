# Settings

The Settings tab: three pages (General, Processes, About) that edit the persisted `Config`. All config mutation lives in one gateway module; the pages only build widgets and wire callbacks.

## Language

**SettingsTab**:
The tab entity: the shared `ConfigStore`, a shared `Arc<AtomicU64>` refresh handle, the `GpuData` enable mirror + re-detect request `Arc<AtomicBool>`s (all live-updating the collector thread), an optional initial page index (from CLI), and the dependency list for the About page.
_Avoid_: "a cloned Config" (the tab holds the store, not a private copy)

**Setting page**:
One of General (refresh interval, theme, window size), Processes (behaviour, columns, default sort), or About (version, license, dependency list). Page order is fixed: indices 0/1/2, matching CLI paths `settings.general/.processes/.about`.
_Avoid_: "tab" (this crate's tab is SettingsTab; the pages are its sub-views)

**Config mutation**:
Editing a typed config field from a UI control via the `mutations.rs` gateway — the ten free functions that parse UI string values into config enums with fallback-to-default semantics for unknown input. They stay pure `&mut Config` functions; call sites wrap them in `store.mutate(|c| …)`, which persists automatically — there is no separate `save()` call.
_Avoid_: writing config from the pages directly; all mutations route through the gateway and the store

**Refresh interval**:
The collector tick rate (500–5000ms). The one setting pushed live to the background thread via the shared atomic — changes apply without restart.
_Avoid_: "poll rate"

**Default View / Resource View / Process Tree Filter / GPU Data**:
The Behaviour settings — `DefaultViewMode` (List/Tree), `ResourceViewMode` (Self/Cumulative), `PidFilterMode` (Direct/All), `GpuData` (On/Off) — surfaced as dropdowns. The GPU Data item pairs the dropdown with a Re-detect button that sets the collector's one-shot re-probe flag, so backends can be rediscovered without relaunching.
**Column settings**:
Visibility toggles and up/down reordering for the nine `SortColumn`s.
**Default sort**:
The persisted sort column + descending switch.

**About page**:
Version/license/author from `env!("CARGO_PKG_*")` (the *settings* crate's metadata, mirrored from the bin) plus a dependency list rendered from `DepInfo`.
**DepInfo**:
One direct dependency's metadata, emitted by the bin's `build.rs` and shared via core's `about.rs`.

## Relationships

- **Settings → Core**: edits `Config`; config field ↔ page/group mapping is the settings domain's core relationship.
- **Settings → Components**: reuses the shared theme menu on the General page.
- **Settings → gpuitop**: `save()` persists to `config.ron`; the refresh atomic is shared with the collector thread.
- **About page**: `CARGO_PKG_*` env vars resolve to the settings crate's Cargo.toml, which must mirror the bin's metadata.
