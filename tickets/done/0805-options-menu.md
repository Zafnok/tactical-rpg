---
id: "0805"
title: "Options menu: speeds, animations, fullscreen, key rebinding"
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: done
blocked_by: ["0405", "0207", "0801", "0208", "0212", "0815"]
nick_input: none
completed: 2026-10-03
---

# 0805 — Options menu

## Context

Players need to tune text speed, animation speed and keys. Default keys and the
right/left-handed layouts are Nick's (`docs/design/controls.md`, [ADR-0015](../../docs/adr/0015-input-actions-and-keymap-layouts.md));
rebinding lets players adjust them.

## Nick input

None.

## Scope

**In:** `Settings` struct persisted via `Storage` key `settings`, Options
screen reachable from title and map menu, a "Key bindings" row that opens 0815's screen.

**Out:** controller bindings (0816, on the Key bindings screen); the key-binding screen itself (0815); the "Map look: Pictures / Glyphs" row (0824, after this ticket and 0436; decided in 0039); the "Language" row (0825) and the "Voices" and "Voice volume" rows (0826), both after this ticket (`docs/design/voices-languages-and-script.md`).

**Audio (added by 0020):** music and sound volume settings, played through the
0212 audio plumbing ([`docs/design/audio.md`](../../docs/design/audio.md)).

## Implementation steps

1. `Settings { version, text_speed: Slow|Normal|Fast|Instant, anim_speed: Normal|Fast, combat_animations: On|Off, enemy_phase_speed: Normal|Fast, auto_end_turn: bool (default false, per `docs/design/turn-structure.md`; also toggled by the `ToggleAutoEnd` key in battle, and that toggle is saved too), fullscreen: bool, layout (right/left-handed; 0208 stores it until now, move it into `Settings`), reset_tips, cursor_style: CursorStyle (Corners (default) | LargeCorners | TileGlow, ticket 0416; move it out of `Ctx::cursor_style`), music_volume: u8, sound_volume: u8 (both 0–10, default 8, *tunable*; 0 = silent; applied by 0212's `app` audio as a multiplier on every cue's own volume) }`.
   Defaults match current behaviour. Loaded at startup into `Ctx`; saved on change.
2. Wire each setting into its consumer (0704 typewriter, 0404 playback, 0502
   pacing, `app` fullscreen via a `FrameOutput` request flag).
3. **Options screen:** list of settings; Left/Right changes value; a "Layout" row switches right/left-handed at any time, as often as the
   player likes (`docs/design/controls.md`): `f` on it opens the same
   `LayoutPickerScreen` as first launch (0208), with the current layout
   focused and Cancel allowed there (it backs out without changing anything);
   a "Cursor" row (Corners / Large corners / Tile glow, `docs/design/look-and-feel.md`);
   picking saves the layout and switches keys immediately, loading **that
   layout's own custom keys** if the player has any (each layout keeps its
   own, `docs/design/controls.md` *Rebinding keys*; 0217's saved
   `keybindings` config); Confirm on "Key bindings" opens 0815's
   `KeyBindingsScreen`; "Reset tips"; "Restore defaults" (all settings;
   custom keys are reset from the Key bindings screen, not here).
4. **Key bindings:** the row pushes `KeyBindingsScreen::new(ctx)`
   (`crates/ui/src/screens/key_bindings.rs`); once it does, remove 0815's
   temporary "Key bindings" entry from the debug menu
   (`crates/ui/src/debug.rs`, `TOOLS` / `KEY_BINDINGS_TOOL`; keep the
   screen's name in `debug::SCREENS`, so the Debug key does nothing while
   it captures a key). No rebinding UI in this ticket. Key bindings are saved
   by 0217 under their own `Storage` key (`keybindings`), not in
   `Settings`, and edited on 0815's screen.
5. **Game mode** (only when a campaign is loaded, per
   `docs/design/death-and-difficulty.md`): shows `Classic` or `Casual`; in
   Classic, `Switch to Casual` asks for confirmation ("This can't be undone")
   and calls `Campaign::downgrade_mode()` (0801). No way back to Classic.
6. Enable `Options` in the map menu and title.

## Acceptance criteria

- [x] Every setting changes behaviour (Harness test per setting where observable).
- [x] Classic → Casual switch works with a confirm; Casual never offers Classic (Harness test).
- [x] Layout can be switched from Options (both directions, repeatedly); the new keys work at once and persist across restart (Harness test).
- [x] "Key bindings" opens 0815's screen; switching layout loads that layout's own custom keys (Harness test).
- [x] Music and sound volume change what `app` plays (Harness test on the requests or the mixer multiplier), 0 silences, and they persist across restart.
- [x] Snapshots of both screens.

## Completion notes

**Done.**

- `Settings` (`crates/ui/src/settings.rs`, ADR-0049): text speed, animation
  speed, combat animations, enemy phase speed, auto-end, fullscreen, cursor
  style, music and sound volume, and the layout picked. Saved as one RON
  record under the `Storage` key `settings` on every change; loaded with
  the storage. A field missing from an older save takes its default.
- `OptionsScreen` (`crates/ui/src/screens/options.rs`), opened from the
  title's new `Options` item and from the map menu's `Options` (now
  enabled). Left/Right change a value and stop at the ends; `Layout` opens
  the layout picker (current layout focused, Cancel backs out);
  `Key bindings` opens 0815's screen; `Game mode` shows only with a
  campaign; `Reset tips`; `Restore defaults`.
- Wiring: the dialogue typewriter reads the text speed; the battle screen
  scales its walk, fight playback, AI action and EXP bar clocks; combat
  animations off skips each fight as Cancel does; the battle's Auto-end key
  and the Options row are the same saved setting; the cursor style comes
  from the settings; `FrameOutput` carries `fullscreen`, `music_volume` and
  `sound_volume`, which `app` applies (`set_fullscreen`, the mixer's
  multipliers, also on the track already playing).
- The layout moved into `Settings`. The old `layout` storage key is still
  read for a player who picked one before, and never written.
- The debug menu's temporary "Key bindings" entry is gone.
- The screen's text is in `assets/lang/en/ui.ron` under `options.*` and
  `title.options` (ADR-0045, which landed on `main` while this PR was
  open), with the test pack's entries.

**Deviations from the plan.**

- `reset_tips` is a row that does something, not a field of `Settings`.
- Confirm (not a fixed `f`) opens the Layout row, as every other row
  (the `keyboard-input` skill).
- Fullscreen goes out in `FrameOutput` as the wanted state every frame, not
  as a one-off request: `app` then needs no separate path for the saved
  value at start-up (ADR-0049).
- The game mode: the Options screen sits on the stack above the game flow,
  which owns the campaign, so it switches the mode through
  `Ctx::campaign_mode`; the flow calls `Campaign::downgrade_mode()` on its
  next frame and sets the battle's setup to Casual for a restart.
- The ticket says both "defaults match current behaviour" and "volume
  default 8 of 10". They can't both hold: with 8 as a multiplier of 0.8,
  the game now plays a fifth quieter than before, and 10 is the old
  loudness.
- Tests had to change where a menu gained a live row: scripted key presses
  in the title, map menu, credits, save, flow, class change, controller
  and key-bindings tests moved by one row.

**Claude's starting rules** (Nick may veto any of them):

1. **Text speed.** Normal is what it was. Slow is half as fast, Fast is
   twice as fast, Instant shows a whole text box at once.
2. **Animation speed: Fast** plays walks, fights and the EXP bar twice as
   fast. Holding Confirm still speeds a fight up four times on top of it.
3. **Enemy phase speed: Fast** plays the enemy's and the other side's turn
   twice as fast. With Animation speed also Fast, four times.
4. **Combat animations: Off** skips every fight the way the Cancel key
   skips one: no fight box, the HP just changes; the EXP bar and level-up
   pages still show.
5. **Volumes** go from 0 (silent) to 10 in steps of one, starting at 8.
   Changing one plays the menu tick, so the new sound volume is heard.
6. **Restore defaults asks yes/no first.** It puts every row back except
   the layout; custom keys are left alone (they have their own Restore
   defaults).
7. **Reset tips** doesn't ask: it says "Tips will show again".
8. **Confirm on a value row** steps to the next value and goes round at the
   end; Left/Right stop at the ends.
9. **Switching to Casual in the middle of a battle:** a unit that falls
   later in that battle does come back for the next one. But in that one
   battle it still says its Classic death quote instead of its retreat
   line, until the battle is restarted. Ticket 0828 fixes that.
10. **The screen's look** is a plain panel in the style of the Key bindings
    screen, with the rows in the ticket's order. No mockups were shown;
    say so if you want to pick from some.

**Follow-up tickets.** 0828 (a switch to Casual reaches the battle under
way).

**For Nick when playing.** Title → Options, and in a battle: map menu →
Options. Try Fullscreen, the two volumes with music playing, Text speed in
a conversation, and Combat animations off in a battle.
