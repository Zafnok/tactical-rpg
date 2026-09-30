# ADR-0031: Player key bindings: slots, per-layout config, fixed Esc/Delete

- **Status:** Accepted
- **Date:** 2026-09-30
- **Related tickets:** 0217, 0030, 0815, 0805, 0218, 0216
- **Supersedes:** the "per-key overrides from Options (0805) apply on top of
  it" part of ADR-0015's *Layouts are data* bullet (the rest of ADR-0015
  stands)

## Context

Nick decided how rebinding works (`docs/design/controls.md`, *Rebinding
keys*, ticket 0030): every action has 3 key slots, some actions must keep a
key and others may have none, a key bound to a second action *moves* there,
each layout (right/left-handed) keeps its own custom keys, `Esc` always
cancels and backs out of "Press a key…", `Delete` empties a slot, and
neither can be bound. ADR-0015 had only planned "per-key overrides on top of
the layout", stored in 0805's `Settings`, which can't express an action
losing its key, 3 ordered slots, or a separate set per layout.

The Key bindings screen (0815) needs a model it can edit without I/O; the
game needs the effective keymap from it at startup and after every edit.

## Decision

- **Defaults stay in `assets/data/keymap.ron`**, now limited to at most
  `SLOTS` (3) chords per action in each layout, in slot order (the first is
  what help text names). The loader rejects a 4th chord, and `Escape` or
  `Delete` (with or without `Shift`) anywhere in the file, including
  `layout_picker`. The layout picker isn't rebindable, so its sections may
  list more than 3. `KeymapDef.layouts` keeps each action's chords in file
  order (`LayoutKeys`); `KeymapDef::bindings` derives the chord lookup.
- **Fixed keys live in `trpg-ui::input::Keymap`, not in data.**
  `Keymap::new` adds plain `Escape` → Cancel to every keymap (layouts, the
  layout picker, player bindings) and drops any chord on a reserved key, so
  `Delete` does nothing in play. `Keymap::chords_for(Cancel)` lists only
  Cancel's own slots; `Keymap::fixed_chords_for` reports `Escape`. `Esc`
  therefore never counts as Cancel's required key.
- **Model** (`trpg-ui::input::bindings`, pure):
  `LayoutBindings { slots: BTreeMap<Action, [Option<Chord>; 3]>, … }` for
  every rebindable action (all but Debug). `bind` puts a chord in a slot and
  empties (and returns) the slot it came from; `clear`, `is_unmapped`,
  `unmapped_required`, `defaults` (also "restore defaults"), `keymap`.
  Invariant: a chord is in at most one slot, and no reserved chord in any
  (property-tested). `Action::is_required` / `is_rebindable` encode the
  design table.
- **Debug** keeps its `keymap.ron` chords and isn't rebindable. In builds
  with debug tools (`DEBUG_TOOLS`, ADR-0023) its chords are reserved; in
  shipped builds a player may bind them, and a slot wins over Debug.
- **Saved config:** `Storage` key `keybindings`, RON:
  ```ron
  PlayerKeys(
      version: 1,
      layouts: {
          "LeftHanded": {
              "Confirm": [Some("j"), Some("Enter"), None],
              …every rebindable action…
          },
      },
  )
  ```
  Names are strings, not serde enums, so one unknown name drops only
  itself. Only layouts that differ from their defaults are written (a
  player who restores defaults gets later default changes). The chosen
  layout stays under `layout`.
- **Repair on load, never fail:** unreadable text or another `version` →
  defaults for every layout. Otherwise, per layout: unknown layouts and
  actions, unreadable chords, extra slots and reserved chords are dropped;
  a chord in two stored slots stays in the first (`Action::ALL`, then slot
  order); a default slot holding a stored chord is emptied (as `bind`
  would); an action not stored keeps its default slots; if a required
  action ends up with no key, that layout goes back to its defaults. Every
  fix is a warning that `Ctx::take_warnings` hands to `app`, which logs it.
- **Wiring:** `Ctx::with_storage` loads `PlayerKeys`; `Ctx::use_layout`
  (and so `choose_layout`) builds the keymap from that layout's player
  bindings. `Ctx::set_layout_bindings` stores, saves and rebuilds the
  keymap if that layout is in use; `Game` compares its input's keymap with
  `ctx.keymap` before and after each frame's update and hands over a changed
  one, so a rebind works from the next key press.
- **Text:** `widgets::help::{key_name, all_key_names, cursor_keys_name}`
  return `String`, `NOT_MAPPED` (`! not mapped`) for an action with no key;
  tips do the same. A caller hides a hint only for its own reason (Rewind
  unavailable, debug tools off). `all_key_names` appends the fixed keys
  (`d/Escape`), so the layout picker's legend looks as before.
- **Keys:** `Key` covers a normal keyboard: punctuation named as itself
  (`,` `.` `/` `'` `[` `]` `\` `-` `=` `` ` ``), `Insert` `Delete` `Home`
  `End` `PageUp` `PageDown`, numpad `Kp0`…`Kp9` `Kp+` `Kp-` `Kp*` `Kp/`
  `Kp.`. Bare modifiers stay unmapped (`Shift` only forms `Shift+` chords).
  `serde` and `ron` become `trpg-ui` dependencies (already used by
  `content`, allowed by ADR-0013).

## Consequences

- 0815 edits a `LayoutBindings` copy and saves it with
  `Ctx::set_layout_bindings`; 0805's layout switch loads each layout's own
  keys through `choose_layout`; 0218 adds its actions as optional ones.
- Changing a default key in `keymap.ron` reaches players who never changed
  that layout; a player's customised layout keeps its saved slots for the
  actions it lists.
- A config from a newer version is reset to defaults (with a warning)
  rather than read partly.
- On the web, miniquad 0.4 doesn't report `'` or `/`, and reports `` ` ``
  as `'`; those keys only bind fully on native.
- Help text can now show `! not mapped` wherever an action has no key,
  including mid-sentence in tips.

## Alternatives considered

- **Overrides on top of the layout in `Settings` (ADR-0015 / 0805's
  plan)** — can't express an action with no key or ordered slots, and
  mixes a per-layout structure into general settings.
- **Esc as a normal default chord on Cancel** — the player could move or
  drop it, and "Esc always backs out" (Nick) couldn't be guaranteed;
  `Esc` would also fill one of Cancel's 3 slots.
- **Serde-derived enums for actions/layouts in the file** — one unknown
  action name (e.g. after an action is renamed) would make the whole file
  unreadable, losing every custom key instead of one.
- **Store only changed actions per layout** — smaller files, but a changed
  default could then collide with a stored key in more ways; storing whole
  layouts keeps what the player saw on the screen.
