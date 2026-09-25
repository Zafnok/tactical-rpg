---
id: "0411"
title: "Battle notes: start-of-battle strategy hints for unique enemies"
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0405", "0801"]
nick_input: sign-off
completed:
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

- [ ] Harness: a test chapter with 2 notes shows the panel before the first phase banner; Confirm closes it; `Objective` lists them again.
- [ ] No panel for a chapter without notes.
- [ ] Validator rejects an unknown unit id (test).
- [ ] Snapshot of the panel.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the validator. Harness and snapshot as above.

## Completion notes
