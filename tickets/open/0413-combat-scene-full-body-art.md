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
an itch artist who has a pack with portraits and battle sprites". 0110 keeps
the bought files in the private assets repo, and 0711 draws bought PNG art
as pixel overlays (ADR-0024). This ticket reuses that drawing for the
fighters.

**Decided in 0021 (2026-09-30), `look-and-feel.md` § Combat screen:** the
art is Mega Tiles' Tiny Tales **big still battle images** (one picture per
hero or class, shipped at 1×, 2× and 3×; use the 1× file), **moved by the
game**: lunge to strike, flash on a hit, shake, fade when defeated (like the
enemies in Final Fantasy VI or Dragon Quest). The packs' small animated
RPG Maker battle sprites are not used. Nick: "1A seems best". Classes with
no hero art use a still image as a stand-in in Chapter 1: church cleric
(Cleric) and church knight (Guard) from *Vol.5 Faith and Evil*, the orc axe
fighter (Brigand) from *Vol.1 Monstrous Uprising*, and an on-foot lance
fighter for the Rider (nothing mounted exists in the catalogue).

## Nick input

**Decision** (use `ask-nick`, with rendered mockups made from the bought
sprites). Ask only what 0021 didn't settle and the packs allow:

- how big the fighters are on screen (the whole-number scales that fit the
  images' real size in the battle screen);
- the motions: how far the lunge goes, flash colour, shake, the miss and
  crit looks (show them as a short GIF or a strip of frames);
- per-class art vs per-character where both exist (the heroes have their
  own still image; generic classes share one);
- which still image stands in for the Rider;
- on/off setting (0805 already has `combat_animations`).

Record the answers in `look-and-feel.md` before implementing.

## Scope

**In:** the design question above; then a `CombatScene` drawn in the
playback overlay: both fighters' bought still images facing each other,
moved by code, HP bars and numbers under each, strike/miss/crit text, per
the answers. Images for the Chapter 1 classes (lord, Rider, Archer, Cleric,
Guard, Mage, Brigand, Raider) mapped from the bought packs.

**Out (do not do):** the packs' small animated battle sprites; drawing
sprites (only the recolours and small pixel edits 0021 allows); images for
classes after Chapter 1 (content tickets per chapter); sound; changes to
combat rules; buying anything.

## Implementation steps

1. Mock up 2–3 options for Nick with the real bought sprites (from
   `assets-private/`, 0110), rendered as in 0011, and record his choice.
2. A battler format next to 0711's PNG portraits: a sidecar per class (and
   per hero that has its own image), `assets/battlers/<id>.ron`, naming the
   one PNG and which way it faces (so the game knows when to mirror it).
   The motions are code, timed by constants in `ui`, not per-image data.
   Validate like 0711 (all errors at once, with file names). Write an ADR
   (`write-adr`).
3. `cargo xtask battler-import`, or extend 0711's `portrait-import`, to copy
   a pack's 1× still image into `assets-private/battlers/` and write the
   sidecar stub.
4. Public placeholders: a small, plain PNG per class that ships in `assets/`
   so a clone without `assets-private/` builds, tests and runs (0110's rule).
   Not meant to look good; Nick only sees the bought art.
5. Draw the scene in 0404's playback overlay with 0711's overlay drawing
   (mirror one fighter so they face each other). Honour the
   `combat_animations` setting (off = 0404's plain box).
6. List each pack in `THIRD_PARTY_ASSETS.md` as ADR-0032 says (marked
   private) and note the class → image mapping in `look-and-feel.md` or the
   class data.

## Acceptance criteria

- [ ] Nick's answers recorded in `look-and-feel.md`.
- [ ] Playback shows both fighters; the setting turns it off.
- [ ] A clone without `assets-private/` builds and passes every gate with
      the placeholders.
- [ ] Snapshots of the scene mid-strike (placeholders), and a rendered PNG
      with the bought sprites sent to Nick.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: battler sidecar parsing and validation, every error with its
  message; the motion curves (position/flash per frame) are pure functions,
  tested frame by frame.
- Snapshot / integration: Harness playback with the scene on and off.

## Completion notes

