---
id: "0706"
title: Chapter 1 cast portraits (bought art, all expressions)
type: content
milestone: M6 Story & dialogue
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0701", "0703", "0011", "0021", "0110", "0711"]
nick_input: sign-off
completed:
---

# 0706 — Chapter 1 cast portraits

## Context

Portraits for every character who speaks in Chapter 1, using the portrait
briefs in `docs/story/characters/*.md` (0701).

**Changed 2026-09-28 and 2026-09-30:** Nick doesn't want Claude-drawn
portraits. In 0021 he chose **Mega Tiles' Tiny Tales packs** for both faces
and combat art (`docs/design/look-and-feel.md`, *Portraits and battle art*;
ADR-0032). This ticket **assigns bought faces** to the cast instead of
drawing them. The files come from the private assets repo (0110) and use
0711's PNG format (48×48 faces, 5 px per pixel).

## First mapping (from the store previews, 2026-09-30)

Made from a few preview images before anything was bought. Previews show
only some of the faces, small, so recheck everything on the real files.

**What the packs have:**
- **Heroes: A New Beginning** ($24.99): 8 heroes with a face set of **8
  expressions** each (which 8 isn't stated on the page), a large portrait,
  a big still battle image and a map sprite. Female Fighter "Child of
  Destiny", Male Fighter "Hero of Prophecy", Archer "Forest Protector",
  Witch, Samurai, Dancer, Thief, Dragon Knight (plus an alternative Dragon
  Knight face).
- **Heroes 2: Rebellious Souls** ($24.99): the same for 8 more, mostly not
  human: female dark elf gladiator, male dark elf mercenary, Amazon
  barbarian, tiefling strider, warforged, female orc, dragonian sorcerer,
  kitsune miko.
- **Still battler packs** (Vol.1–5): battle images and map sprites only,
  **no faces**. Generic enemies have no faces anywhere in the catalogue.
- **Character Generator EX** ($49.99, early access): makes new characters in
  the same style with 8 expressions, a battle image and a map sprite.

**Candidates:**

| Character | Best match | Fit |
| --------- | ---------- | --- |
| `lead_m` | Male Fighter "Hero of Prophecy" (sword, long coat) | Good; recolour hair if needed |
| `lead_f` | Female Fighter "Child of Destiny" | Good; check it matches `lead_m`'s costume and colours |
| `poacher` Aske | Archer "Forest Protector" | Partial: check he doesn't read as an elf |
| `rival` Dace | Samurai (dark hair) or the dark elf mercenary | Partial |
| `heretic` Rue | Thief | Partial: a rogue's face, not hunched in a big coat |
| `retainer` Hollis | none (old, broad, mail coif) | **Gap** |
| `red_captain` Harl | none (big, bearded, kettle helm) | **Gap** |
| `keeper` Piers | none (round, balding, grey hood) | **Gap** |
| `sergeant` Tamsin | none (cavalry cape, riding cap) | **Gap** |
| `vowmaster` Crane | none (hood up, spectacles) | **Gap** |
| `soldier` (generic) | none | **Gap** |

**Gaps** (Nick, 0021: "probably A or D"): first the Character Generator EX,
*after* Claude has confirmed its licence allows generated characters in a
sold game and Nick has bought it; otherwise, or on top, Claude's small
edits to a bought face. Commissioning, or changing a character's look to fit
a face (a story change), needs asking Nick first.

**What Claude can and can't do to bought faces:** recolour hair, clothes and
eyes (a palette swap of a few exact colours), and small pixel edits (a scar,
a missing `surprised` made from a neutral face, a spectacle rim). Not new
hairstyles, removing beards, or new clothes: that's redrawing, and Nick
doesn't want Claude's art.

## Nick input

**Sign-off:** for each Chapter 1 speaker, Claude proposes two or three
candidate faces from the bought packs (rendered in the dialogue screen,
neutral plus one other expression), and Nick picks one or asks for others.
Characters no pack fits are handled as in *Gaps* above.

## Scope

**In:** one portrait per Chapter 1 speaking character (the player-gendered
lead gets **two**: `lead_m` and `lead_f`, per `setting-and-tone.md`), with
the five required expressions mapped to the pack's expressions, plus any
extra ones listed in the character sheet. One shared `soldier` portrait for
unnamed enemies.

**Out:** later chapters; drawing new art (unless 0021 allows edits, and then
only the edits it allows).

## Implementation steps

1. List Chapter 1 speakers from `docs/story/chapters/ch01.md`.
2. For each, read the portrait brief and shortlist two or three bought
   faces that match it (age, build, class, colours).
3. Render the candidates in the dialogue screen (a rendered PNG, as in 0704)
   and send them to Nick. Record his picks.
4. Import each pick with `cargo xtask portrait-import` (0711) into
   `assets-private/portraits/`. Map `neutral`, `happy`, `angry`, `sad` and
   `surprised` to the closest pack expressions, and note the mapping in the
   character sheet.
5. Update `docs/story/characters/*.md` with which pack and face each
   character uses. List each pack in `THIRD_PARTY_ASSETS.md` (marked private).

## Acceptance criteria

- [ ] Every Chapter 1 speaker has a validated portrait with all required
      expressions.
- [ ] Characters are distinguishable in a greyscale screenshot.
- [ ] Nick approved each pick.
- [ ] All gates in the `run-gates` skill pass.

## Completion notes

