---
id: "0706"
title: Chapter 1 cast portraits (bought art, all expressions)
type: content
milestone: M6 Story & dialogue
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0701", "0703", "0011", "0021", "0035", "0110", "0711"]
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

**Waits for 0035** (added 2026-10-01): Harl's face must match his combat
picture, and 0035 picks that picture (and may change his class, which 0803
needs to know). Take Harl's face from 0035's answer.

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
- **Character Generator EX** ($49.99, early access; also in the $99.99
  bundle): makes new characters in the same style: a face set with 8
  expressions, a small animated battle sprite (not a big still image) and a
  map sprite. Its licence allows generated characters in a sold game.
  Content is thin so far: few outfits and hairstyles; casters, helmets and
  glasses are still promised.

**Candidates:**

| Character | Best match | Fit |
| --------- | ---------- | --- |
| `lead_m` | Male Fighter "Hero of Prophecy" (sword, long coat) | Good; recolour hair if needed |
| `lead_f` | Female Fighter "Child of Destiny" | Good; check it matches `lead_m`'s costume and colours |
| `poacher` Aske | Archer "Forest Protector" | Partial: check he doesn't read as an elf |
| `rival` Dace | Samurai (dark hair) or the dark elf mercenary | Partial |
| `heretic` Rue | **Witch** (decided in 0021: face and combat picture match) | Rewrite her portrait brief to the Witch's look, hair recoloured as needed |
| `retainer` Hollis | none (old, broad, mail coif) | **Gap** |
| `red_captain` Harl | none (big, bearded, kettle helm) | **Gap** |
| `keeper` Piers | none (round, balding, grey hood) | **Gap** |
| `sergeant` Tamsin | none (cavalry cape, riding cap) | **Gap** |
| `vowmaster` Crane | none (hood up, spectacles) | **Gap** |
| `soldier` (generic) | none | **Gap** |

## Checked on the bought files (2026-10-02, ticket 0038)

Nick bought the whole Mega Tiles bundle. The files are sorted on his
machine in `D:\tactical-rpg\Tiny Tales Bundle Assets\`; open `index.html`
there to see each character's faces, battle picture and map sprite
together. This replaces the preview-based notes above where they differ.

- **Expressions**, the same 8 for every hero and for every face the
  generator makes: `neutral`, `smile`, `stern`, `sad`, `surprise`,
  `thinking`, `sly`, `unique`. So `happy` → smile, `angry` → stern,
  `sad` → sad, `surprised` → surprise map directly; `sly` and `thinking`
  are spare for the sheets' extra expressions (`smug`, `sneer`, `cold`,
  `weary`…). Each exists as a 48×48 face and as an 80×80 bust (face or
  bust in dialogue: ticket 0039, question 7).
- **Faces exist only for the 16 heroes**, plus what the Character
  Generator EX 1.2 makes. Its parts: six kinds of outfit (soldier, rogue,
  brawler, commoner, traveller, simple dress), one wizard hat, many hair
  styles, beards and moustaches; **no hoods, helmets, coifs or glasses**.
- **What the real art shows**, against the table above:

| Character | On the real files |
| --------- | ----------------- |
| `lead_m`, `lead_f` | Male and Female Fighter: good. He wears a blue coat with steel shoulder plates; she wears pink and carries a curved sabre, so their costumes don't match each other |
| `poacher` Aske | The Archer is plainly a **female elf** (long blonde hair, pointed ears, a leaf in her hair). Not a 17-year-old boy. A generator face, or a pack from 0040 |
| `rival` Dace | Samurai: dark blue hair in a ponytail, red coat, a long katana; reads as young and slight. Partial. The Magitek *Human Noble* still picture (dark hair, blue coat, sword) fits his look better but has no face |
| `heretic` Rue | Witch: silver-blue hair, a big blue hat, a staff and an orb. As decided |
| `retainer` Hollis | Still picture: *Faith and Evil* Church Knight (full helm, spear, tower shield): good for a Guard. Face: generator (soldier outfit, grey hair, full moustache); no coif |
| `keeper` Piers | Still picture: Church Cleric (hooded, green-haired, reads as a young woman) or Church Wizard (a boy with a staff). Neither is a soft man of 34. Face: generator, no hood |
| `sergeant` Tamsin | **Nothing mounted exists in all 37 products.** On foot, the *Magitek Dynasty* Dynasty Soldier (red and white, spear, red hair) is closest. Face: generator |
| `vowmaster` Crane | Still picture: Church Archmage (old, mitre, white beard) or *Gods and Gallants* "Dark Sovereign Okuul" (a tall pointed hood, a scythe). Face: generator can't do the hood or spectacles |
| `red_captain` Harl | No human axe fighter except the Amazon Warrior hero (a woman). Ticket 0035 |
| `soldier` (generic) | Faces only from the generator; map sprites and still pictures for bandits, soldiers and church wardens exist |

- **Later cast, found while looking:** the Heroes 2 **Amazon Warrior**
  (a big axe; face, still picture and map sprite) fits Hedda Ravn, the
  Brennish Raider of Chapter 5.
- **Fighters with no fitting art get more bought packs** (Nick,
  2026-10-02: "help me locate some more itch bundles to fill in with a
  similar style for rest of cast... or if these are non-battle units, I
  think the generator can work"): ticket 0040. A character who fights
  takes their face from the same pack as their combat picture where that
  pack has faces; a character who never fights may use a generator face.
  **Either order works with 0040:** if 0040 is done, use the packs Nick
  bought. If it isn't, give Aske, Piers, Tamsin and Hollis generator
  faces now, tell Nick in the sign-off which ones are stand-ins, and add a
  line to 0040 to swap them when a pack is bought.

**Gaps** (Nick, 0021: "probably A or D"): first the Character Generator EX
once Nick has bought it (its licence was checked in 0021: commercial use is
fine); in combat a generated character uses its class's still image;
otherwise, or on top, Claude's small
edits to a bought face. Nick also allows **rewriting a character's written
look to fit the bought art** (his words: "we can rewrite Rue's physical
description as needed once we buy the art"): update the character sheet's
portrait brief in the same PR, and list each rewrite in the PR for Nick.
No commissions (Nick vetoed them).

**Face and combat picture match** (0021): a character with their own hero art
uses it for both; everyone else uses their class's combat picture,
recoloured to their colours (0413).

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

