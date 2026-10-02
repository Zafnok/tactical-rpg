---
id: "0805"
title: "Options menu: speeds, animations, fullscreen, key rebinding"
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0405", "0207", "0801", "0208", "0212", "0815"]
nick_input: none
completed:
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

**Out:** controller bindings (0816, on the Key bindings screen); the key-binding screen itself (0815); the "Map look: Pictures / Glyphs" row (0824, after this ticket and 0436; decided in 0039).

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

- [ ] Every setting changes behaviour (Harness test per setting where observable).
- [ ] Classic → Casual switch works with a confirm; Casual never offers Classic (Harness test).
- [ ] Layout can be switched from Options (both directions, repeatedly); the new keys work at once and persist across restart (Harness test).
- [ ] "Key bindings" opens 0815's screen; switching layout loads that layout's own custom keys (Harness test).
- [ ] Music and sound volume change what `app` plays (Harness test on the requests or the mixer multiplier), 0 silences, and they persist across restart.
- [ ] Snapshots of both screens.

## Completion notes

