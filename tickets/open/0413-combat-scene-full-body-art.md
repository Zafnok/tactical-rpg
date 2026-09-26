---
id: "0413"
title: Combat scene with full-body art of both combatants
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: high
status: todo
blocked_by: ["0404", "0703"]
nick_input: decision
completed:
---

# 0413 — Combat scene with full-body art

## Context

In ticket 0011 Nick said the combat screen "probably" should show "a full body
rendering of the 2 battling units", rather than portraits
(`docs/design/look-and-feel.md`). 0404 builds combat playback as a text box
with names and HP bars. This ticket adds a scene with the two fighters drawn
full-body, in the same half-block pixel-art technique as portraits
(ADR-0018), shown while the strikes play out.

## Nick input

**Decision** (use `ask-nick`, with rendered mockups like 0011's): whether
he wants it at all (he said "probably"), art size (e.g. 32×48 px per
fighter), per-class art vs per-character, how much animation (static poses,
2–3 frame lunge, hit flash), and an on/off setting (0805 already has
`combat_animations`). Record the answers in `look-and-feel.md` before
implementing.

## Scope

**In:** the design question above; then a `CombatScene` drawn in the playback
overlay: both fighters' full-body art facing each other, HP bars and numbers
under each, strike/miss/crit text, per the answers. Placeholder art for the
Chapter 1 classes.

**Out (do not do):** final art for every class (content tickets per chapter);
sound; changes to combat rules.

## Implementation steps

1. Mock up 2–3 options for Nick (render them with the real font atlas, as in
   0011) and record his choice.
2. Extend the portrait pixel format (0703) or add a sibling `.sprite` format
   for full-body frames (size, frames, colour keys). If it's a new format,
   write an ADR.
3. Draw the scene in 0404's playback overlay; honour the
   `combat_animations` setting (off = 0404's plain box).
4. Placeholder art for the Chapter 1 classes (lord, Rider, Archer, Cleric,
   Guard, Mage, Brigand, Raider).

## Acceptance criteria

- [ ] Nick's answers recorded in `look-and-feel.md`.
- [ ] Playback shows both fighters; the setting turns it off.
- [ ] Snapshots of the scene mid-strike.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: sprite parsing/validation (if a new format).
- Snapshot / integration: Harness playback with the scene on and off.

## Completion notes

