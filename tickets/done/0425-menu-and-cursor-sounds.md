---
id: "0425"
title: "Menu and map-cursor sounds"
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: done
blocked_by: ["0212", "0213"]
nick_input: sign-off
completed: 2026-09-29
---

# 0425 — Menu and map-cursor sounds

## Context

Nick chose our own menu sounds in ticket 0020
([`docs/design/audio.md`](../../docs/design/audio.md)). They are rendered by
0213 as the cues `menu_move`, `menu_select`, `menu_cancel`, and
`cursor_move`, which is `menu_move` at 60 % volume (*tunable*).

On the map cursor, Nick said "fire emblem has it for the cursor so I think we
can have it for the cursor as well at a lower vol maybe?".

## Nick input

**Sign-off:** click through menus and move the map cursor around, including
holding a key down. Does the tick get tiring? Is the cursor volume right?

## Scope

**In:**
- The shared menu widget (`crates/ui/src/widgets/menu.rs`): moving the
  highlight → `menu_move`; confirming an enabled item → `menu_select`;
  backing out → `menu_cancel`.
- Every screen that handles Confirm/Cancel without the menu widget (title,
  battle action flow, forecast, info screens) uses the same three cues for
  the same meanings. Grep for `Action::Confirm` / `Action::Cancel` handling.
  Advancing dialogue text is **not** a menu action: it plays nothing (Nick
  wasn't asked).
- The battle map cursor: each tile it moves → `cursor_move`. Key repeat
  (ticket 0421) ticks on every repeated step.
- Selecting a disabled menu item plays **nothing**. This is *Claude's starting
  rule*: list it in the PR for Nick.

**Out (do not do):**
- Battle event sounds (0424), music (0807, 0814).
- New sounds or cues.

## Implementation steps

1. Add the three calls in `widgets/menu.rs` and cover them with the widget's
   tests.
2. Walk every screen's input handling and add the cues where Confirm, Cancel
   or cursor movement change something. No sound when the input does nothing,
   e.g. moving past the edge of a list or the map.
3. The battle cursor: `cursor_move` per tile moved, including camera-scroll
   moves.

## Acceptance criteria

- [x] Harness tests: menu move, select, cancel and disabled select emit the
      right requests (or none). (Disabled select: see the notes.)
- [x] Harness test: holding the cursor key for 1 s emits one `cursor_move`
      per tile moved, and none at the map edge.
- [ ] Nick signed off. (Pending: asked for in the PR.)
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: menu widget.
- Snapshot / integration: Harness tests on title, battle cursor and one
  menu-heavy screen.

## Completion notes

- **Menu widget:** `Menu::handle_with_sound(action, &mut ctx.audio)` plays
  `menu_move` when the focus moves, `menu_select` on a chosen item and
  `menu_cancel` on Cancel; nothing otherwise. `Menu::without_cancel()` is
  for menus with nothing to back out of (title, layout picker), so their
  Cancel stays silent. `trpg_ui::audio::MenuSound` names the three sounds
  for screens that don't use the widget.
- **`cursor_move`** is a manifest entry: `menu_move.wav` at volume 60
  (*tunable* in `assets/audio/audio.ron`).
- **Screens wired:** title, Coming-soon placeholder, layout picker, debug
  menu, glyph sampler, portrait viewer, dialogue skip prompt, and the whole
  battle screen: map cursor (one tick per tile, camera scrolls included;
  key repeat ticks every step; nothing at the edge), the action, weapon,
  skill, item, equip, map and unit-list menus, targeting / forecast, the
  end-turn prompt, objective, info screen and the rewind screen.
- **How the battle screen decides** (`screens/battle/sounds.rs`): its menus
  are pure data stepped by `mode::step`, so the sound is read from what
  the step changed. Nothing changed → nothing plays.
- **Disabled select:** in every real menu the highlight skips disabled
  items, so a disabled item can only be focused when *nothing* is enabled.
  That case is covered by the widget's unit tests (`a_disabled_item_is_denied`);
  the Harness test covers the nearest real case, Confirm on a unit that has
  already acted, which plays nothing.
- Harness gained `sounds()` (the sound cues played, without music).
- **Nick's review of the starting rules (2026-09-29):** rules 2 and 4-7
  kept. Rule 1 changed: a greyed-out item plays a warning, `menu_cancel`
  for now (`MenuSound::Denied`), later its own tone. Rule 3: silent for
  now; Nick may want a whoosh when a message closes and will judge after
  playing.
- Follow-ups: 0031 (decide the warning tone and the message-close whoosh),
  0427 (render and wire them).

**Claude's starting rules** (the design docs don't cover these; Nick can
veto any of them):

1. ~~Selecting a disabled menu item plays nothing.~~ Nick: it plays a
   warning. For now the cancel sound; its own tone comes with 0031/0427.
2. Opening something plays the select sound, even when the back key opens
   it: back on the map opens the map menu, and back during a conversation
   opens "Skip this scene?". Both play select, not cancel.
3. Closing a message plays nothing, like reading on through dialogue: tips,
   the phase and victory/defeat banners, the EXP bar and the level-up page.
   (Nick: fine for now; maybe a whoosh later, see 0031.)
4. Skipping a unit's walk or a fight's animation plays nothing.
5. "Next unit" jumping the map cursor to a unit plays one cursor tick, not
   one per tile it jumps over.
6. Picking a target (the forecast) or a unit on the info screen uses the
   menu move sound, not the map cursor tick: it's choosing from a list.
7. The info key sounds like select when it opens the info screen and like
   cancel when it closes it. The danger-zone and auto-end toggles play
   nothing.
