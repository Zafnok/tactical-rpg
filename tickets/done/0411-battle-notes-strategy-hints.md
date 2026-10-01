---
id: "0411"
title: "Battle notes: start-of-battle strategy hints for unique enemies"
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: done
blocked_by: ["0405", "0801"]
nick_input: sign-off
completed: 2026-10-01
---

# 0411 — Battle notes

## Context

Nick (0004, `docs/design/magic.md` "Battle notes") liked how Fortune's Weave
shows a recommended strategy at the start of a battle ("stun this monster",
"use this trebuchet"). He wants the same when facing a unique enemy such as
an elemental. Notes are chapter data (0801's chapter file) shown by the battle
screen.

## Nick input

**Sign-off:** Nick reads the notes panel on a test chapter and comments on
wording and placement.

## Scope

**In:**
- `battle_notes` in the chapter file and `BattleSetup`.
- The start-of-battle notes panel.
- Re-reading the notes from the map menu's `Objective` page.
- Highlighting the units a note is about.

**Out (do not do):**
- Writing Chapter 1's actual notes (0803).
- Automatically generated hints.

## Implementation steps

1. Chapter file (0801 format, document it in `assets/chapters/README.md`):
   `battle_notes: [ (text: "Frost Elemental: weak to Fire, absorbs Ice.", units: ["frost_elemental_1"]) ]`.
   Default `[]`. The validator checks that the unit ids exist.
2. `BattleSetup.battle_notes` (plain data, no rules effect).
3. The battle screen shows a centred boxed panel `BATTLE NOTES`, listing the
   notes, after Preparations and before the first `PLAYER PHASE` banner. It
   closes on Confirm. While it is open, the named units blink/highlight. It
   is skipped when there are no notes.
4. Map menu `Objective` page: list the notes under the objective.

## Acceptance criteria

- [x] Harness: a test chapter with 2 notes shows the panel before the first phase banner; Confirm closes it; `Objective` lists them again.
- [x] No panel for a chapter without notes.
- [x] Validator rejects an unknown unit id (test).
- [x] Snapshot of the panel.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the validator. Harness and snapshot as above.

## Completion notes

Done. A battle file can carry `battle_notes`; the battle screen shows them
in a `BATTLE NOTES` box when the battle starts and lists them again on the
map menu's `Objective` page; the units they are about blink meanwhile.

- **Core:** `BattleNote { text, units }` in `BattleDef`, `BattleSetup` and
  `BattleState` (`battle_notes()`); plain data, saved with the state.
- **Content:** `battle_notes` in the battle file; an enemy or reinforcement
  entry may take `id: "name"` so a note can point at it. The loader checks
  the notes (`crates/content/src/battle.rs`, tests in `battle/tests.rs`).
  Documented in `assets/battles/README.md`.
- **UI:** `crates/ui/src/screens/battle/notes.rs`, `Queued::Notes` in the
  battle screen's queue; tests in `notes_tests.rs` and
  `crates/ui/tests/flow.rs` (two snapshots).
- The test chapter's battle (`assets/battles/test.ron`, what New Game plays)
  has two **placeholder** notes, so the box can be seen on the Pages build.
  The Quick Battle has none.

**Deviations from the ticket**

1. The notes are in the **battle file**, not the chapter file. ADR-0035
   (written after this ticket) moved a battle's units into
   `assets/battles/`, and a note names those units; world-map battles
   (1007/1008) have no chapter. `assets/chapters/README.md` points there.
2. Acceptance says "before the first phase banner". Today a battle shows
   **no** `PLAYER PHASE` banner on turn 1 (0405 left it to 0801, which
   didn't add it), so the first banner after the notes is `ENEMY PHASE`.
   The notes are first in the screen's queue, so the turn-1 banner will
   come after them once it exists: **follow-up ticket 0430** (added to the
   playtest's blockers and the roadmap's critical path).
3. Preparations (0408) isn't built; the notes show at the battle's start.
   0408 pushes its screen from the flow before the battle screen, so the
   order "Preparations, then notes" needs nothing more here.
4. The box isn't always centred: see rule 2 below. On the test map a
   centred box would have covered the brigand it is about.

**Claude's starting rules** (the design was silent; Nick may veto; also in
`docs/design/magic.md`):

1. The box comes before anything else in the battle: a turn-1 scene and the
   first-battle tip wait for it. Only Confirm closes it; other keys do
   nothing. Retry and `Restart Battle` show it again.
2. The box sits in the middle of the map; if a unit it is about would be
   hidden under it, it moves to the top of the map (or the bottom).
3. Highlight: a unit a note is about swaps its letter and background
   colours, 0.4 s on, 0.4 s off. It also blinks while the `Objective` page
   is open.
4. `Objective` page: the objective, the turn, a blank line, `Battle notes`,
   then the notes. Each note starts with `•`.
5. At most 5 notes per battle, 120 characters each (so they always fit).
6. The camera doesn't move to a noted unit that is off screen.

**For Nick's sign-off:** New Game on the Pages build → skip the intro scene
→ the `BATTLE NOTES` box. The Brigand should blink. Press Confirm, then
open the map menu → `Objective`. Comments wanted on the wording style, the
box's place, and the blink.
