# 01 — Configurable pause keybind

**What to build:** the user can change which key combo pauses/resumes collection from Settings instead of being stuck with the hardcoded Esc. Today the pause key is a constant in the app shell and the status bar always says "Esc: Pause"/"Esc: Resume". This ticket makes the combo a persisted interface setting (default `escape`), editable on the General → Interface settings page via a click-to-capture control, and wired into the app shell's key handler and the status-bar hint.

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

- [ ] Interface config gains a pause-key field (keystroke syntax string, e.g. `ctrl-shift-p`), defaulting to `escape`, persisted via the config store (existing mutate path)
- [ ] Settings → General → Interface shows the current binding as a `Kbd` element; clicking it captures the next pressed combo (Esc alone cancels)
- [ ] The app shell parses the configured combo with `Keystroke::parse` and toggles pause on an exact modifiers+key match, falling back to `escape` on an invalid value
- [ ] Status-bar hint renders the configured combo with the `Kbd` element
- [ ] `--override` can set the combo (falls out of the existing config model)

Follow existing conventions: mutation goes through the settings mutations gateway and `ConfigStore::mutate`; the App shell reads config live (no mirror copies).
