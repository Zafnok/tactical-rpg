---
id: "0434"
title: "Battle tests learn the state from the map scene, not from cells"
type: infra
milestone: M3 Battle UI
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0432"]
nick_input: none
completed:
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

- [ ] The grep from step 1 finds, outside `map_view/`, only tests listed in
      the PR as look tests, each with a one-line reason.
- [ ] No test was deleted or weakened: the PR lists each moved test with
      what it asserted before and after.
- [ ] No `.snap` file changed.
- [ ] The rule is in `crates/ui/README.md` and the `work-ticket` skill.
- [ ] All gates in the `run-gates` skill pass (the mutation gate still
      kills the mutants the old assertions killed; check the changed
      files' mutants locally if the gate can't tell).

## Tests required

- Unit: the new helpers (on a small scripted battle).
- Property: none.
- Snapshot / integration: the moved tests themselves.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
