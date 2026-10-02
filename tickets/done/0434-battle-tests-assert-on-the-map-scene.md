---
id: "0434"
title: "Battle tests learn the state from the map scene, not from cells"
type: infra
milestone: M3 Battle UI
model: opus-5.5
effort: medium
status: done
blocked_by: ["0432"]
nick_input: none
completed: 2026-10-02
---

# 0434 — Battle tests assert on the map scene

## Context

ADR-0038, rule 4: tests about *behaviour* ("the unit moved", "these tiles
are in range", "the cursor is here") assert on the `MapScene` or the
`BattleState`; only tests about a *look* read the frame. Today many battle
tests learn the state from the glyph look: the two letters in a cell, a
cell's background colour, the cursor's overlay rectangles at hard-coded
cell coordinates (e.g. `crates/ui/tests/it/battle.rs`: `buf.get(36, 18)`,
`.overlays()`, the helper that reads "the two glyphs drawn on the tile";
ADR-0024: "tests find the cursor from its overlays").

While the glyph skin is the default those tests pass, so this isn't
urgent. But they are what would turn "change the default skin" from a
small job into a week of test rewrites, and they break on any tuning of
the glyph look. 0432 gave the Harness `map_scene()` and `map_text()`.

## Nick input

None.

## Scope

**In:**
- Battle tests in `crates/ui/tests/` and
  `crates/ui/src/screens/battle/*_tests.rs` (and `mod.rs`'s test module)
  that read cells, cell colours or overlays inside the map area to learn
  game state: move them to scene or state assertions.
- A small set of scene helpers for tests.
- The rule written where test authors will see it.

**Out (do not do):**
- Tests of a look: `units.rs`, `cursor.rs`, `path.rs` unit tests (now
  under `map_view/glyph*`), the font-coverage tests
  (`every_glyph_drawn_is_in_the_font`), and every insta frame snapshot.
  They stay as they are.
- Tests that read text in panels, menus or the help bar (`shows(&h,
  "ENEMY PHASE")`): that text is the same in every skin.
- Changing what any test checks. Same intent, different source.

## Implementation steps

1. List the tests to move:
   `grep -n "buffer()\.get\|\.overlays()\|\.glyph\b\|\.bg\b" crates/ui/tests
   crates/ui/src/screens/battle`. For each hit decide: *look* (leave) or
   *state* (move). Put the list in the PR description.
2. Add test helpers beside the Harness (`crates/ui/src/harness.rs`, or a
   `harness::battle` submodule), built on `map_scene()`:
   `cursor_tile() -> Option<Pos>`, `unit_at(Pos) -> Option<UnitView>`,
   `tints_at(Pos) -> Vec<RangeKind>`, `path() -> Vec<Pos>`.
3. Rewrite each *state* test with them, or with `BattleState` through
   `flow()` where the scene adds nothing. Keep each test's name.
4. Add to `crates/ui/README.md` (testing section) and to the `work-ticket`
   skill's checklist: "A test that checks what happened reads the scene or
   the state. Only a test of a look reads cells, colours or items, and it
   lives with the skin."
5. If 0433 is done: run the moved tests under the sprite skin too (a loop
   over skins in the shared setup), since they no longer depend on the
   look. If it isn't, add a line to 0433's steps to do that.

## Acceptance criteria

- [x] The grep from step 1 finds, outside `map_view/`, only tests listed in
      the PR as look tests, each with a one-line reason.
- [x] No test was deleted or weakened: the PR lists each moved test with
      what it asserted before and after.
- [x] No `.snap` file changed.
- [x] The rule is in `crates/ui/README.md` and the `work-ticket` skill.
- [x] All gates in the `run-gates` skill pass (the mutation gate still
      kills the mutants the old assertions killed; check the changed
      files' mutants locally if the gate can't tell).

## Tests required

- Unit: the new helpers (on a small scripted battle).
- Property: none.
- Snapshot / integration: the moved tests themselves.

## Completion notes

**Done.** Battle tests that learned the game state from the glyph look now
read the map scene (or the state). Helpers: `Harness::cursor_tile()`,
`unit_at(pos)`, `tints_at(pos)`, `path()`; `MapScene::cursor_tile()`,
`unit_at(pos)`, `tints_at(pos)`, `terrain_at(pos)`, each with unit tests.
The rule is in `crates/ui/README.md` (*Writing a Harness test*) and in the
`work-ticket` skill. No `.snap` file changed.

**Moved** (test: before → after):

- `tests/it/battle.rs`
  - `quick_battle_opens_on_the_player_phase_banner`, `arrows_move_the_cursor_and_the_panel_follows`, `cancelling_the_menu_then_the_selection_restores_the_unit`: cursor's corner-mark overlays at cell x → `cursor_tile()`.
  - `select_move_and_wait_dims_the_unit_and_keeps_its_label_case`: `Lo` at cells (30,16), fg dimmed, `..` at (26,16) → `unit_at((5,5))` is `Lo`, acted; (3,5) empty.
  - `cancelling_the_menu_…`: `Lo`/`╦╦` cells → `unit_at` on (5,5) and (3,5).
  - `confirm_on_an_enemy_toggles_its_range`: bg of cell (36,18) changes → `tints_at((8,7))` is `[Attack]`, then empty.
  - `a_full_turn_of_waits_ends_with_auto_end_…`, `a_full_turn_of_waits_then_space_…`: `Lo` at (26,16) → lord shown, not acted (and all 4 player units ready).
  - `the_lord_fights_the_near_brigand_on_turn_one`: `Lo` at (32,15) → `unit_at((6,4))` is `Lo`, acted.
  - `the_rogue_arrives_and_turn_three_opens_with_a_scene`: `╦╦`/`Ro` at (44,14) → `unit_at((12,3))` empty / `Ro`.
- `screens/battle/mod.rs`
  - `cursor_pulses_with_frame_time`: 8 overlays in cursor colour, full / scaled 0.5 → scene cursor on (3,5) at brightness `1.0` / `BLINK_MIN`.
  - `held_cursor_ticks_once_per_tile`, `harness_keys_move_the_cursor`, `harness_hold_repeats_with_the_keymap_timing_and_scrolls`: cursor cell from overlay arms → cursor's tile offset in the view (same numbers, halved x).
  - `large_map_scrolled_to_bottom_right_snapshot`: `R` at cell (68,29) → that unit at (63,39), the view's last tile (34,29).
  - `selecting_draws_ranges_and_a_double_panel_border`: 8 cursor overlays + cell (26,16), bg tints, no path overlay → cursor on (3,5) as corners, `tints_at` Move / Attack, path only the lord's tile. (The panel border check stays: panel.)
  - `the_path_ends_in_an_arrowhead_with_no_cursor_frame_there`: path overlay counts, no cursor marks, `·Lo..╦╦..` → path (3,5)→(5,5), no cursor, lord on (3,5), fort empty; then cursor on (13,5). (The arrowhead look is `map_view/glyph/path.rs`.)
  - `the_menu_opens_beside_the_unit_drawn_at_its_new_tile`: `....Lo`, `.` under the cursor → lord on (5,5), (3,5) empty, no cursor, no ranges. (Menu text/colours stay.)
  - `an_enemys_threat_area_is_tinted_until_hidden`: bgs of 3 cells → `tints_at` (8,2), (8,7) `[Attack]`, then empty.
  - `a_pending_move_after_an_attack_is_chosen_on_the_map`: bg of a tile → `tints_at` `[Move]`.
- `ai_phase_tests.rs` `an_action_pans_marks_the_unit_then_walks`: overlay count drops → scene cursor on the unit, then none.
- `art_tests.rs` (pinning shot): bg behind `Br` differs → brigand `has_effect` true / false.
- `attack_tests.rs` `after_the_playback_the_defenders_hp_is_the_battles`, `a_kill_fades_the_unit_out_…`: `Lo`/`Br`/`..` cells → `unit_at` (acted; 0 HP not fading; gone).
- `magic_tests.rs` (6 tests + the Harness fire test): tile glyphs `**`/`▒▒`/`,,`/`♣♣` and bg tints/flash colours → the terrain shown (`becomes` or own), `tints_at`, the tile's `flashes` (1.0, 0.5, none).
- `notes_tests.rs` `the_noted_unit_blinks_while_the_notes_are_up`: tile colours swap → the unit's `highlight` on/off.
- `skill_tests.rs` effect colour test, shove test: label bg → `has_effect`; raider `tints_at` `[Attack]`.
- `trigger_tests.rs` `a_fallen_rogue_says_its_last_words_before_it_fades`: `Ro`/`..` cells → rogue shown, fall barely begun (fade < 0.25), then gone.
- `turn_tests.rs` danger zone test: each tile's bg → `tints_at` `[Danger]` exactly in the zone; `[Danger, Move]` under a selection.

**Looks kept on the buffer** (left by the grep, each a look or non-map
text): every `row`/`text`/`shows`/`help`/`panel`/`glyphs` helper (panel,
menu, help-bar and box text, the same in every skin); `every_glyph_drawn_is_in_the_font`
and `assert_in_font` (font coverage); `mod.rs` `draw_covers_the_whole_buffer`
(nothing stale anywhere), `terrain_tiles_use_their_two_glyphs_and_colours`
(the glyph skin's terrain look), the panel border and HP-bar glyphs, the
menu's colours; `forecast.rs`, `playback.rs`, `progress.rs` (forecast,
combat box, EXP/level-up box looks); `scene_tests.rs` (what a test skin
paints and where menus open for it: the skin-swap mechanics);
`item_tests.rs` heal popup (the `+10` text and colour, drawn by the screen
beside the tile); `info.rs`/`panel.rs`/`forecast.rs:102` (drawing code,
not tests).

**Deviations.** The effect colour behind a unit was only checked by the
battle tests, so its look moved into a new glyph skin test
(`a_unit_under_an_effect_has_the_effect_colour_behind_its_letters`). The
other looks the old assertions checked are already covered in
`map_view/glyph*` (acted dimming, cursor pulse, path arrowhead, highlight,
tints, flashes). Step 5: 0433 isn't done, so its step 6 now says to run
these tests under both skins. Map tiles read through helpers the grep
didn't match (`tile(…)`, `text(…, 2)`, `*_CELL` constants) were moved too.

No gameplay rules decided. Nothing for Nick to play: no game change.
