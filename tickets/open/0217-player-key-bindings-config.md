---
id: "0217"
title: "Player key bindings: 3 slots per action, per-layout saved config, fixed Esc"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: high
status: todo
blocked_by: ["0216"]
nick_input: none
completed:
---

# 0217 — Player key bindings config

## Context

Nick's rebinding rules are in [`docs/design/controls.md`](../../docs/design/controls.md),
*Rebinding keys* (ticket 0022). This ticket builds the model and saved
config behind them; the screen that edits it is 0809, and 0805's Options
menu opens that screen.

Today (ADR-0015, tickets 0204/0208) `assets/data/keymap.ron` holds each
layout's bindings, `Keymap::for_layout` builds the active keymap, and the
chosen layout is saved under the `Storage` key `layout`
(`crates/ui/src/screen.rs`, `LAYOUT_KEY`). 0805 planned a
`key_overrides: BTreeMap<Action, Vec<Chord>>` in `Settings`; this ticket
replaces that plan with its own saved config (0805 is updated to match).

0216 made sure nothing reads or names keys outside the keymap, so the
effective keymap built here reaches every screen, help bar and tip.
Follow the `keyboard-input` skill.

## Nick input

None. (Rules already decided in 0022.)

## Scope

**In:**
- 3 slots per action in the defaults and in the player's config.
- Required vs optional actions (`controls.md` table); Debug not rebindable.
- `Esc` fixed as Cancel, `Delete` reserved; neither can be in a slot.
- Player config saved per layout under the `Storage` key `keybindings`.
- A pure editing API for 0809 (bind with "key moves" conflicts, clear,
  restore defaults, list unmapped required actions).
- More `Key`s, so "any key the game can read" covers a normal keyboard.
- `! not mapped` in help bars and tips for an action with no key.
- ADR for the config format and the fixed keys.

**Out (do not do):**
- The Key bindings screen (0809) and the Options menu (0805).
- The new Select / Confirm end turn actions (0218).
- Controller or mouse input.
- Changing any default key other than moving `Escape` out of Cancel's list
  into the fixed binding (it keeps working exactly as before).

## Implementation steps

1. **Actions** (`crates/content/src/keymap.rs`): add
   `Action::is_required(self) -> bool` (true for `CursorLeft`, `CursorDown`,
   `CursorUp`, `CursorRight`, `Confirm`, `Cancel`, `EndTurn`) and
   `Action::is_rebindable(self) -> bool` (false only for `Debug`), with the
   list documented as coming from `controls.md`.
2. **More keys**: add `Key` variants and names for everything macroquad
   reports on both native and web that a player might bind: `, . / ' [ ] \
   - = \``, `Insert`, `Delete`, `Home`, `End`, `PageUp`, `PageDown`, numpad
   `0`–`9` and `+ - * / .` (`Kp0`…), and map them in `crates/app/src/keys.rs`.
   Pick chord names that `Chord::parse` can read back (e.g. `"Comma"` if a
   bare `","` is awkward in RON); round-trip test every `Key`. Bare modifier
   keys (`Shift`, `Ctrl`, `Alt`, `Super`) stay unmapped: `Shift` only forms
   `Shift+` chords.
3. **`keymap.ron` defaults**: at most 3 chords per action (loader error
   otherwise); `Escape` and `Delete` may not appear (loader error: "Esc and
   Delete are fixed, see controls.md"). Remove `"Escape"` from both layouts'
   `Cancel`. Update the header comment. Keep every other key as is.
4. **Fixed keys** (`crates/ui/src/input.rs`, `Keymap`): plain `Escape`
   always maps to `Cancel` in every keymap, including the layout picker's.
   `Delete` maps to nothing in play (0809's screen reads it as a raw key
   while editing). `Keymap::chords_for(Cancel)` does **not** include
   `Escape`; add `Keymap::fixed_chords_for(action)` so 0809 and help can show
   it. Debug's chords stay as in `keymap.ron` and are reserved (can't be
   bound by the player) in builds with the debug tools feature (ADR-0023).
5. **Slots model** (new `crates/ui/src/input/bindings.rs`, or a module next
   to `input.rs`; pure, no I/O):
   ```rust
   pub const SLOTS: usize = 3;
   pub type Slots = [Option<Chord>; SLOTS];
   /// One layout's player-edited bindings: every rebindable action's slots.
   pub struct LayoutBindings { slots: BTreeMap<Action, Slots> }
   impl LayoutBindings {
       pub fn defaults(def: &KeymapDef, layout: Layout) -> Self;
       pub fn slots(&self, a: Action) -> Slots;
       /// Puts `chord` in `a`'s slot `i`. If `chord` was in another slot
       /// (any action, including `a`), that slot is emptied and returned.
       pub fn bind(&mut self, a: Action, i: usize, chord: Chord)
           -> Result<Option<(Action, usize)>, BindError>; // BindError::Reserved for Esc/Delete/Debug's
       pub fn clear(&mut self, a: Action, i: usize);
       pub fn is_unmapped(&self, a: Action) -> bool;       // all slots empty
       pub fn unmapped_required(&self) -> Vec<Action>;     // in Action::ALL order
       pub fn keymap(&self, repeat: RepeatDef) -> Keymap;  // + fixed Esc, + Debug
   }
   ```
   Invariant: a chord is in at most one slot; reserved chords are in none.
6. **Saved config** (`Storage` key `keybindings`, next to `LAYOUT_KEY`):
   RON, `PlayerKeys { version: 1, layouts: BTreeMap<Layout, BTreeMap<Action,
   Slots>> }`. A layout with no entry uses its defaults; an action missing
   from a stored layout uses its default slots. **Each layout keeps its own
   keys**: switching layout loads that layout's entry. On load, repair
   rather than fail: drop unknown actions, reserved chords and duplicate
   chords (the stored action keeps it; a clashing default slot is emptied,
   same as `bind`); if the result leaves a required action unmapped, reset
   that layout to its defaults. Unreadable or wrong-version data → defaults
   for all layouts, and a warning in the log (no crash).
7. **Wire it in**: at startup `Game` builds the active keymap from the saved
   layout + `PlayerKeys` instead of `Keymap::for_layout`. Add
   `Ctx::player_keys()` and `Ctx::set_layout_bindings(layout, LayoutBindings)`
   (saves and rebuilds `InputState`'s keymap at once) for 0809, and make
   switching layout (today only the first-launch picker; later 0805) load
   that layout's saved bindings.
8. **`! not mapped` in text** (`crates/ui/src/widgets/help.rs`, `tips.rs`):
   `key_name` / `all_key_names` / `cursor_keys_name` return `! not mapped`
   (constant `NOT_MAPPED`) when the action has no key; tips placeholders do
   the same. Check every caller that used `None` to hide a hint still hides
   it only for its own reasons (e.g. Rewind's `can_open_rewind` filter).
   Help bars that show Cancel list its slots, not `Esc` (unchanged look).
9. **ADR** (`write-adr` skill): ADR-0028 "Player key bindings: slots,
   per-layout config, fixed Esc/Delete", superseding ADR-0015's "per-key
   overrides from Options (0805) apply on top" bullet. Record the storage
   format and the repair-on-load rule.
10. Update `docs/design/controls.md`'s Bindings table note that `Esc` is now
    a fixed key (no rule change), and the `keyboard-input` skill if any
    name here differs from it.

## Acceptance criteria

- [ ] `keymap.ron` loader rejects 4 chords on one action, and `Escape` or
      `Delete` in any list (tests).
- [ ] `Esc` cancels in both layouts and the layout picker even though it is
      in no slot (Harness test); `Esc`/`Delete` can't be bound
      (`BindError::Reserved`, unit test).
- [ ] Binding a key that another action has moves it and leaves that action
      `is_unmapped` when it was its only key (unit test); binding within the
      same action moves between slots.
- [ ] Property test: after any sequence of `bind`/`clear` on random actions,
      slots and chords, no chord is in two slots and no reserved chord is in
      any slot.
- [ ] `unmapped_required` lists exactly the required actions with no key.
- [ ] Custom keys persist across restart (MemoryStorage round trip) and are
      kept **per layout**: edit right-handed, switch to left-handed and back,
      the right-handed edit is still there (test).
- [ ] Corrupt, old-version or clashing stored config loads repaired, never
      panics (tests for each case).
- [ ] A help bar and a tip show `! not mapped` for an unbound action (tests).
- [ ] Every new `Key` round-trips through its chord name; `app` maps each.
- [ ] ADR-0028 written; ADR index updated.
- [ ] `cargo xtask check-keys` (0216) passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: loader limits and reserved keys; `LayoutBindings` bind/clear/move/
  unmapped; config repair cases; `NOT_MAPPED` in help and tips.
- Property: the no-duplicate/no-reserved invariant (proptest).
- Integration: Harness — rebind via `Ctx::set_layout_bindings`, the new key
  works at once and the old one doesn't; `Esc` still cancels; restart keeps
  the binding.

## Completion notes
