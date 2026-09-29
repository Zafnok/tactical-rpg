---
id: "0022"
title: "Decide: EXP bar and level-up sounds"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 0022 — Decide: EXP bar and level-up sounds

## Context

Ticket 0602 added the EXP bar, the level-up screen and the class-progress
box (`crates/ui/src/screens/battle/progress.rs`), silent for now: its scope
said "sound effects (future audio ticket)". No ticket covered them, and
[`docs/design/audio.md`](../../docs/design/audio.md) lists "level up" under
*Places and moments without a cue yet* ("Nick hasn't been asked about
these"). Ticket 0424 says not to invent sounds that `audio.md` doesn't list.
So Nick has to pick them before 0605 can play them.

The moments on screen (see the 0602 snapshots in
`crates/ui/src/screens/battle/snapshots/*progress_tests*`):

1. **EXP bar filling:** about 0.6 s. Fire Emblem plays a rising tick while
   the bar fills; Advance Wars and Final Fantasy Tactics play nothing.
2. **The bar reaching 100** (the level up), just before the level-up page.
3. **The level-up page opening** (`LEVEL UP!`): FE plays a short fanfare
   jingle; Tactics Ogre and Triangle Strategy play a short chime.
4. **Each stat that grew appearing** (one every 0.12 s): FE plays a tick per
   `+1`.
5. **Class mastered** (`CLASS MASTERED!`) and **Learned: <skill>**: maybe a
   bigger version of 3, or nothing.

Constraints (ADR-0013): CC0-1.0, CC-BY-4.0 or made by us, nothing that costs
money. The in-house chip family (`menu_select`, `heal` "HE5") already exists
(`docs/design/audio.md`), so rendered options fit next to it; library
options need their license checked and a `THIRD_PARTY_ASSETS.md` row.

## Nick input

**Decision.** Nick listens to an audition page (as in ticket 0020) and picks
a sound, or silence, for each of the five moments.

## Scope

**In:**
- Using the `ask-nick` skill: an audition page with several options per
  moment (in-house renders like 0020's `MENU` and `HEAL` families, and a few
  CC0/CC-BY-4.0 library picks), each labelled with the game it's like.
  Always offer "nothing" and "describe your own".
- Record the answers in `docs/design/audio.md`: new cue rows (e.g.
  `exp_tick`, `level_up`, `stat_up`, `class_mastered`) and Nick's words in
  the appendix. Remove "level up" from the open sub-questions.

**Out (do not do):**
- Playing the sounds in the game: that's ticket 0605.
- Victory/defeat stings, shops and other open cues in `audio.md`.

## Implementation steps

1. Read `docs/design/audio.md`, ticket 0020 (its audition-page approach) and
   the 0602 snapshots.
2. Build the audition page and ask Nick with `ask-nick`, scoped to the five
   moments above.
3. Record the answers in `audio.md` (the cue table, the source, the
   license).

## Acceptance criteria

- [ ] `docs/design/audio.md` names a cue, or says "no sound", for each of the
      five moments, with Nick's words.
- [ ] Every chosen library sound's license is CC0-1.0 or CC-BY-4.0.

## Tests required

- None (design decision).

## Completion notes

