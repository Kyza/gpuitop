# 05 — One config home

Status: resolved
Type: task
Blocked by: —

## Goal

One shared `Config` handle all tabs read, with the mutations gateway as the
sole writer; the hand-sync mirror handlers and the "restart to apply"
surprises disappear.

## Friction

Persisted state is copied into three homes:

- `App.config` (`crates/gpuitop/src/app_view.rs:29`).
- `SettingsTab.config` (`crates/settings/src/tab.rs`).
- `ProcessesTab.column_visibility` + `clear_search_on_pin`
  (`crates/processes/src/tab.rs`) — a *slice* of config.

Hand-sync leakages:

- The theme-menu commit handler manually mirrors a theme change into both
  `App.config` and `settings_tab.config` (`app_view.rs:371-391`); the
  settings page's own theme handler (`settings/src/general.rs:97-106`) does
  the same to a different copy.
- Settings edits to column visibility or default sort never reach the running
  processes tab — only tab construction reads them. A user changes columns in
  Settings and it doesn't apply until restart.

Dead persisted fields with no runtime consumer: `default_grouping`
(`core/src/config.rs:159`), `disk_devices` (:161), `network_interfaces`
(:163). `load_from(path)` (~198-216) duplicates `load()` (~235-257).

## Solution sketch

- A single shared `Config` handle (`Rc<RefCell<Config>>` or similar) read by
  the App, SettingsTab, and ProcessesTab.
- `mutations.rs` gateway is the sole writer; one invalidation path.
- The tab subscribes to config changes instead of snapshotting config at
  construction.
- Prune the dead persisted fields; unify the load paths.

## Wins

- locality: one mutation path, one save point.
- settings changes apply live, not after restart.

## Acceptance

- [x] Changing columns/default-sort in Settings applies to the running
      processes tab without restart.
- [x] Theme change has exactly one mutation path (no mirror handler).
- [x] `load`/`load_from` deduplicated; dead fields removed.

## Answer

Implemented. Design settled by grilling:

- **ConfigStore** (`core/src/config_store.rs`): `Rc<RefCell<Config>>` +
  generation; `get()` / `mutate(f)` (apply + bump + auto-save) /
  `generation()`. One write path; `mutate` persists.
- **ProcessSeeds** (`core/src/processes/seeds.rs`): per-field diff baseline
  for `default_sort` + `default_view_mode`, re-applied at the top of
  `ProcessesTab::render` via `apply_seed_changes` (only on actual change).
- **Wiring**: `main.rs` builds the store; `App.config`, `SettingsTab.config`
  are `ConfigStore`; all settings `this.save()` calls deleted; dropdown value
  closures read `store.get()`.
- **Live columns**: core delegate's `column_visibility: ProcessesConfig`
  field replaced by the store; `is_col_hidden` reads it per cell.
- **Theme**: `theme_menu`'s internal clone-save removed (it persisted a stale
  copy); the `config` param is gone from `build_theme_menu`; persistence
  lives in each caller's `on_commit` → `store.mutate`. Titlebar mirror
  handler deleted.
- **Prune + dedup**: `default_grouping`, `disk_devices`,
  `network_interfaces` removed; `load()` folded into `load_from(None)`.
- **Non-goals kept**: `window_size` startup-only; `resource_view_mode` /
  `pid_filter_mode` stay toolbar-owned seeds; CLI `--tree/--list` wins at
  construction.
- Tests: 9 new (ConfigStore mutate/persist/clone-share; ProcessSeeds diff);
  delegate tests construct with `ConfigStore::new(Config::default())`.
  Full workspace: 272 passing.
