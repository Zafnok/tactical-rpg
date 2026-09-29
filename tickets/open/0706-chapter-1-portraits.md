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

**Changed 2026-09-28:** Nick doesn't want Claude-drawn portraits. He chose
bought art, CaptainSkolot's portrait bundle (0021 records the decision and
its rules). This ticket now **assigns bought portraits** to the cast
instead of drawing them. The files come from the private assets repo (0110)
and use the 64×64 PNG format (0711).

## First mapping (from the store previews, 2026-09-28)

Claude went through every pack in the bundle's store pages before Nick
bought it. Previews are small, so recheck everything on the real files.

**What the bundle has:**
- **15 one-character packs** with 9–12 expressions each: Knight, Women
  Knight, Bearded Knight, Wise Old Knight (grey beard), Man Peasant, Woman
  Peasant, Blacksmith (9), Lumberjack, Wise Wizard, Old Villager, Blond
  Villager (man and woman), Medieval Lady (black hair, red dress),
  Merchant, Little Girl. Their sheets show neutral, smiles and laughs, sad,
  angry, serious, smirk, wink and an open-mouthed shocked face (Nick
  confirmed the Bearded Knight's bottom-left face reads as surprised).
- **The Ultimate Medieval Village pack:** 12 characters with **6
  expressions each and no "surprised"** (some also lack "sad"). The seller's
  expression lists are on its store page.
- **Crowd packs** (soldiers, women soldiers, women, vikings, pirates, elves,
  orcs, "magic heroes" 1–3, Gameboy): one face per character, no
  expressions. The soldier packs are modern military. Good only for generic
  portraits.
- Off-theme packs (cowboys, astronauts, animals, mafia, Christmas, chefs):
  not used.

**Candidates:**

| Character | Best match | Fit |
| --------- | ---------- | --- |
| `retainer` Hollis | Wise Old Knight (12) | Strong, almost as-is |
| `red_captain` Harl | Bearded Knight or Lumberjack (12), hair and beard recoloured to rust red | Good with a recolour |
| `lead_m` | Knight (12), already brown-haired | Good face; plate armour instead of the travel coat |
| `lead_f` | Women Knight (12), recoloured blond → dark chestnut | Good with a recolour; armour again |
| `poacher` Aske | Man Peasant (12) recoloured to straw blond, or the village peasant boy (6) | Good with a recolour; no braids or hood |
| `rival` Dace | Village Young Knight (dark hair, dark armour) | Looks right; missing `sad` and `surprised` |
| `keeper` Piers | Village Monk (bald, bearded), robe recoloured grey | Weak: bald rather than thinning hair; no `surprised` |
| `heretic` Rue | Medieval Lady (12, black hair, red) | Weak: a refined noble, not a hunched, squinting rogue |
| `sergeant` Tamsin | Village Seamstress (auburn) at best | **Gap** |
| `vowmaster` Crane | Wise Wizard (big white beard) at best | **Gap** (he's clean-shaven, with a hood and spectacles) |
| `soldier` (generic) | Any vikings or men-soldier face | Fine (one expression is enough) |

Later chapters: `king` could be the village Knight Commander or the Old
Villager (partial); `sister` Wren could be the village peasant girl
(partial).

**What Claude can and can't do to bought faces:** recolour hair, clothes and
eyes (a palette swap of a few exact colours), and small pixel edits (a scar,
a missing `surprised` made from a neutral face, a spectacle rim). Not new
hairstyles, removing beards, or new clothes: that's redrawing, and Nick
doesn't want Claude's art. Gaps are resolved per Nick's answer to 0021
question 3.

## Nick input

**Sign-off:** for each Chapter 1 speaker, Claude proposes two or three
candidate faces from the bought packs (rendered in the dialogue screen,
neutral plus one other expression), and Nick picks one or asks for others.
Characters no pack fits follow Nick's answer to 0021 question 3.

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

