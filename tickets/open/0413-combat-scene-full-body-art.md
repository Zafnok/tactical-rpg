---
id: "0413"
title: Combat scene with bought full-body art of both combatants
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: high
status: todo
blocked_by: ["0404", "0021", "0035", "0110", "0711", "0231"]
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
as sprite items (0231, ADR-0038; changed 2026-10-01 from per-pixel
overlays). This ticket reuses that drawing for the fighters: each fighter
is one sprite item, moved by changing its `dest`.

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
Named characters without their own hero art use their class's picture
recoloured to their colours (Nick: "sure"); a character's face and combat
picture must match (Rue uses the Witch for both). The boss **Harl**'s
picture is decided in ticket 0035 (Nick shops around), which this ticket
waits for since 2026-10-01 (before that, nothing would have put his picture
in once it was chosen). Use 0035's pick; if Nick chose to keep a stand-in,
use the stand-in he named. The picture's weapon must match the unit's
weapon type (weapon types have their own rules).

Nick wants this scene in the Chapter 1 playtest (2026-10-01), so 0804 waits
for this ticket.

**Changed 2026-10-02 (ticket 0038).** Nick bought the whole Mega Tiles
bundle and saw two mockups of this scene made from the real files, with a
bought battle background behind the fighters: G1a, in today's 44-cell box
over the map, art at its own size (a 336×248 background fits the box
almost exactly), and G1b, filling the screen with the art at double size.
His words: "for G1a and G1b I think both are a little off. The main thing
that's off though is that you put the enemy facing backwards... let's
always mirror the image to face the player when battling. Other than
that, I worry G1b won't be able to convey all the info, but maybe it can
work with some additional in-battle overlays or pop up messages".

- **Decided: the two fighters always face each other**; a picture facing
  the wrong way is mirrored (`look-and-feel.md`, *Combat screen*). Every
  still battle picture looked at on the bought files (the heroes and the
  Vol.1, 4 and 5 classes) **faces right as bought**, so the fighter on the
  right is the one to mirror. Check each picture when importing it; the
  sidecar's "which way it faces" field (step 2) is what the game reads.
- **Still open, ask in this ticket:** box or full screen, and if full
  screen, what overlays or pop-up messages carry the information the box
  shows today (names, HP, hit, damage, crit, the strike text); whether a
  bought background is drawn behind the fighters at all. Show both layouts
  again with the facing fixed.
- **What the bundle has for backgrounds:** *Battlebacks Vol.1*, 24
  pictures of 336×248: desert, oasis, two fields, meadows, two mountains
  and wasteland, each at day, dusk and night. Nothing indoors, no forest
  interior, no town, no bridge or water. If backgrounds are in, which one
  a fight uses follows the terrain the defender stands on and the map's
  light (ticket 0438: noon → day, dusk → dusk, night → night); say which
  terrains have no fitting picture and what they fall back to.
- **Sizes found:** hero pictures are 64–128 wide and 72–80 tall; class
  pictures 48–128 by 48–96; the *Gods and Gallants* and *Epic Monsters*
  pictures are much bigger (up to 221×184), so a boss from those towers
  over a hero at the same scale.
- The still battle pictures are sorted in the private assets repository
  (ADR-0040; `cargo xtask private-assets --library`), in
  `assets-private/library/tiny-tales/characters/` (`heroes/<Name>/
  battler.png`, `battler-classes/<pack>/<Name>/battler.png`).

## Nick input

**Decision** (use `ask-nick`, with rendered mockups made from the bought
sprites). Ask only what 0021 didn't settle and the packs allow:

- where the scene is shown: the box over the map or the full screen, with
  the facing fixed, and what tells the player the numbers in the
  full-screen version (see *Changed 2026-10-02* above);
- whether a bought battle background is drawn behind the fighters;
- how big the fighters are on screen (the whole-number scales that fit the
  images' real size in the battle screen);
- the motions: how far the lunge goes, flash colour, shake, the miss and
  crit looks (show them as a short GIF or a strip of frames);
- per-class art vs per-character where both exist (the heroes have their
  own still image; generic classes share one);
- which still image stands in for the Rider;
- how the recolours look for named characters (show two or three);
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
   `assets-private/library/`, ADR-0040), rendered as in 0011, and record
   his choice.
2. A battler format next to 0711's PNG portraits: a sidecar per class (and
   per hero that has its own image), `assets/battlers/<id>.ron`, naming the
   one PNG and which way it faces (so the game knows when to mirror it).
   The motions are code, timed by constants in `ui`, not per-image data.
   Validate like 0711 (all errors at once, with file names). Write an ADR
   (`write-adr`).
3. `cargo xtask battler-import`, or extend 0711's `portrait-import`, to copy
   a pack's 1× still image into `assets-private/game/battlers/` and write the
   sidecar stub.
4. Public placeholders: a small, plain PNG per class that ships in `assets/`
   so a clone without `assets-private/` builds, tests and runs (ADR-0040).
   Not meant to look good; Nick only sees the bought art.
5. Draw the scene in 0404's playback overlay with sprite items, as 0711
   does (mirror one fighter with `flip_x` so they face each other). If a
   motion Nick picks needs something a sprite item can't do yet (a flash
   to white, a tint), add it to `Sprite` here (the item, `app`'s renderer,
   the snapshot line and, if done, 0232's PNG renderer) and note it in
   ADR-0038's section 2; never fall back to per-pixel rectangles. Honour the
   `combat_animations` setting (off = 0404's plain box). The setting comes
   from the Options menu (0805), which this ticket doesn't wait for: if
   0805 isn't done, keep the value in `Ctx` (as `cursor_style` and
   `text_speed` are today, default on) and add a line to 0805 to move it
   into `Settings`.
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

