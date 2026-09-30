---
id: "0413"
title: Combat scene with bought full-body art of both combatants
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: high
status: todo
blocked_by: ["0404", "0021", "0110", "0711"]
nick_input: decision
completed:
---

# 0413 — Combat scene with bought full-body art

## Context

In ticket 0011 Nick said the combat screen "probably" should show "a full body
rendering of the 2 battling units", rather than portraits
(`docs/design/look-and-feel.md`). 0404 builds combat playback as a text box
with names and HP bars. This ticket adds a scene with the two fighters drawn
full-body while the strikes play out.

**Changed 2026-09-30:** the art is **bought**, not drawn by Claude. Nick
doesn't want Claude-drawn character art (0021). His words: "I guess we need
an itch artist who has a pack with portraits and battle sprites". 0021 picks
the artist and packs, 0110 keeps the bought files in the private assets repo,
and 0711 draws bought PNG art as pixel overlays (ADR-0024). This ticket
reuses that drawing for battle sprites.

## Nick input

**Decision** (use `ask-nick`, with rendered mockups made from the bought
sprites). Ask only what 0021 didn't settle and the packs allow:

- whether he wants the scene at all (he said "probably");
- how big the fighters are on screen (the scale options that fit the
  sprites' real size in the battle screen);
- how much animation: static poses, or the pack's own frames (attack, hit,
  dodge) as far as the pack has them;
- per-class art vs per-character (depends on what the pack sells);
- on/off setting (0805 already has `combat_animations`).

Record the answers in `look-and-feel.md` before implementing.

## Scope

**In:** the design question above; then a `CombatScene` drawn in the playback
overlay: both fighters' bought sprites facing each other, HP bars and numbers
under each, strike/miss/crit text, per the answers. Sprites for the Chapter 1
classes (lord, Rider, Archer, Cleric, Guard, Mage, Brigand, Raider) mapped
from the bought packs.

**Out (do not do):** drawing sprites (only the recolours and small pixel
edits 0021 allows); sprites for classes after Chapter 1 (content tickets per
chapter); sound; changes to combat rules; buying anything.

## Implementation steps

1. Mock up 2–3 options for Nick with the real bought sprites (from
   `assets-private/`, 0110), rendered as in 0011, and record his choice.
2. A sprite format next to 0711's PNG portraits: a sidecar per class,
   `assets/sprites/<class>.ron`, naming the PNG frames for each pose
   (`idle`, `attack`, `hit`, … as the pack provides) and the frame timing.
   Validate like 0711 (all errors at once, with file names). Write an ADR
   (`write-adr`).
3. `cargo xtask sprite-import`, or extend 0711's `portrait-import`, to copy a
   pack's frames into `assets-private/sprites/` and write the sidecar stub.
4. Public placeholders: a small, plain PNG per pose that ships in `assets/`
   so a clone without `assets-private/` builds, tests and runs (0110's rule).
   Not meant to look good; Nick only sees the bought art.
5. Draw the scene in 0404's playback overlay with 0711's overlay drawing
   (mirror one fighter so they face each other). Honour the
   `combat_animations` setting (off = 0404's plain box).
6. List each sprite pack in `THIRD_PARTY_ASSETS.md` (marked private) and
   note the class → sprite mapping in `look-and-feel.md` or the class data.

## Acceptance criteria

- [ ] Nick's answers recorded in `look-and-feel.md`.
- [ ] Playback shows both fighters; the setting turns it off.
- [ ] A clone without `assets-private/` builds and passes every gate with
      the placeholders.
- [ ] Snapshots of the scene mid-strike (placeholders), and a rendered PNG
      with the bought sprites sent to Nick.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: sprite sidecar parsing and validation, every error with its message.
- Snapshot / integration: Harness playback with the scene on and off.

## Completion notes

