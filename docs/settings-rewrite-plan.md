# Settings UI Rewrite Plan

Replace the hand-rolled settings UI (`src/tabs/settings.rs`) with
`gpui-component`'s `Settings`/`SettingPage`/`SettingGroup`/`SettingItem` components.

## Architecture

- `SettingsTab` stores a shared `Rc<CloneCell<Config>>` (or pass config to `SettingField`
  getter/setter closures via `cx.entity()` reads).
- `refresh_ms: Arc<AtomicU64>` stays for live refresh-interval changes.
- Theme: `Rc<Cell<Theme>>` passed to `SettingsTab::new()`. A `Theme` dropdown in settings
  updates both the `Rc<Cell<Theme>>` and saves the config, mirroring the
  header theme button.
- `Reset to defaults` button per page: resets all fields on that page to `Config::default()`.
- `Settings::new("gpuitop-settings").pages(...)` renders sidebar + search.

## Pages

### 1. General (`IconName::Settings2`)
- **Refresh interval** — dropdown: 0.5s / 1.0s / 1.5s / 2.0s / 3.0s / 5.0s
- **Theme** — dropdown: Dark / Light / System. Updates `Rc<Cell<Theme>>` + saves.

### 2. Processes (`IconName::Cpu`)
- **VRAM polling** — dropdown: Auto / On / Off
- **Process tree filter** — dropdown: Direct only / All descendants
- **Clear search on pin** — switch (bool)
- **Resource view** — dropdown: Self / Cumulative

### 3. Columns (`IconName::Columns`)
- One `switch` per column:
  - Process ID, User, State, CPU usage, Memory usage, VRAM usage,
    Disk read, Disk write, Full command
  - Name is always on (locked/resettable per page but no toggle).
  - Keywords: each item gets `column`, `toggle`, column name.

### 4. About (`IconName::Info`)
- Static info: version, description, link(s).

## Implementation Steps

1. **Pass `Rc<Cell<Theme>>`** to `SettingsTab::new()`. Store it as a field.
2. **Rewrite `src/tabs/settings.rs`**:
   - Remove `section()`, `setting_row()`, `toggle_button()`, and all `render_*` methods.
   - Add a `setting_pages(&self, window, cx)` method returning `Vec<SettingPage>`.
   - Implement `Render` using `Settings::new(...).pages(self.setting_pages(...))`.
   - Each `SettingItem::new(title, SettingField::dropdown/switch(...))`
     with getter/setter closures reading/writing `self.config` via
     `cx.entity().read(cx)` / `cx.entity().update(cx, ...)`.
   - Theme field: both updates the `theme` cell and saves config.
3. **Update `app.rs`**: pass `self.theme.clone()` to `SettingsTab::new()`.
4. **Verify it builds and runs**. Navigate settings, toggle values, verify they persist.
5. **Write tests** for process search and filter logic (see below).
6. **Remove** dead code (old helpers from settings.rs).

## Migration of each setting

| Setting | Current widget | New widget |
|---|---|---|
| Refresh interval | toggle buttons | `SettingField::dropdown` |
| VRAM polling | toggle buttons | `SettingField::dropdown` |
| Process tree filter | toggle buttons | `SettingField::dropdown` |
| Clear search on pin | toggle buttons | `SettingField::switch` |
| Resource view | toggle buttons | `SettingField::dropdown` |
| Column toggles | custom ON/OFF buttons | `SettingField::switch` each |
| About | raw text | `SettingItem::render` with static content |
| Theme (new) | header button | `SettingField::dropdown` in General page |

## Test Plan: Process Search & Filters

Add tests in `src/tabs/processes.rs` (or a new test module) covering:

### Fuzzy search tests
- Exact match scores highest
- Partial match scores lower
- Case-insensitive matching
- No match returns false/None
- Consecutive character matches score higher than gapped
- Unicode handling (if applicable)

### Filter tests
- Each `Filter` variant correctly filters a known set of `ProcessInfo` entries
  (GUI, User, System, Systemd, Kernel, Parent, VRAM, Electron, ProcessState, Pid)
- `FilterMode::And` — only processes matching ALL filters pass
- `FilterMode::Or` — processes matching ANY filter pass
- Empty filter list passes all processes
- Combining search text + filters: search + filter work together

### Integration
- `filtered_sorted_rows` snapshot: given a known list of processes, search text,
  filters, sort column/direction, verify correct rows returned in correct order.
- Sort stability: changing sort column reorders correctly.
- Search ranking: rows with better fuzzy matches sort first.
