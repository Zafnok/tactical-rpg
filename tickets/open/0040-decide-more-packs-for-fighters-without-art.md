---
id: "0040"
title: "Decide: more bought packs for fighters with no fitting art"
type: design-decision
milestone: M3 Battle UI
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0038"]
nick_input: decision
completed:
---

# 0040 — Decide: more bought packs for fighters with no fitting art

## Context

Nick bought the whole Mega Tiles bundle on 2026-10-02 (ticket 0038). On
the real files some fighters have no fitting art (`tickets/open/
0706-chapter-1-portraits.md`, *Checked on the bought files*). Nick: "for
the cast, I guess you need to help me locate some more itch bundles to
fill in with a similar style for rest of cast... or if these are
non-battle units, I think the generator can work".

So: **characters who never fight** get a face from the Character
Generator EX (bought), and this ticket isn't about them. **Characters and
classes that fight** need a still battle picture, a map sprite and, if
they speak, a face, and where the bundle has none, Claude finds packs in
a similar style and Nick decides each purchase (ADR-0032 rule 7: Nick
buys; Claude rechecks price and licence first; no price ceiling is asked).
No commissions, no Claude-drawn art (`look-and-feel.md`).

**The gaps** (what each needs: P = still battle picture, M = map sprite,
F = face):

| Gap | Who | Needs | Stand-in today |
| --- | --- | ----- | -------------- |
| 1. Mounted fighters | Rider (Tamsin, Chapter 1), Lancer, Outrider, Iron Rider and their tier-3 classes; later a flying mount (Flier) | P, M | An on-foot lance fighter (`look-and-feel.md`) |
| 2. Human axe fighter | Harl (Chapter 1 boss, ticket 0035), Brigand and Raider enemies | P (F for Harl) | The orc axe fighter; human `Warrior_M*` map sprites exist |
| 3. Male archer | Aske (Chapter 1) | P, F | The Archer hero, who is plainly a female elf |
| 4. Hooded male cleric; old hooded priest with spectacles | Piers (Chapter 1); Crane (enemy caster, Chapter 7) | P, F | Church Cleric / Church Archmage; the generator has no hoods or glasses |
| 5. Fist fighter | Brawler (Gil Parrow, Chapter 4) | P | None; the generator has a brawler outfit for face and map sprite |
| 6. A rival swordsman in black | Dace (Chapter 1 tease, Chapter 4) | P, F | The Samurai hero (partial) |

What a pack must match: Tiny Tales is hand-drawn pixel art with dark
outlines and saturated colours; faces 48×48 (or busts 80×80); still battle
pictures 64–130 pixels tall; map sprites 16 wide by 20 tall on 16×16
tiles. A sprite drawn for 32-pixel tiles is twice too big; art with
realistic proportions clashes with the chibi map sprites.

## Search results (2026-10-02)

About 55 store pages were read on 2026-10-02 (itch.io first, then the
Komodo / RPG Maker store, GameDev Market and OpenGameArt). Nothing was
bought or downloaded. Prices and licence quotes are as the pages showed
that day; step 1 rechecks them. Style is scored 1–5 against Tiny Tales
from the store previews.

**What the search found, plainly:**

- **Mega Tiles sells nothing we don't own.** The store lists 37 products
  and the bundle is the same 37. There is no mounted pack, no human axe
  pack, and no add-on parts for the Character Generator.
- **The artists named in our packs don't sell a fix either.** Kodots Games
  Studio sells one monster pack; Rayane Félix and Lunatic Red sell
  nothing; Aekashics sells many human fighters (bandits, mounted samurai)
  but as large painted pictures, not pixel art.
- **Size is the recurring problem.** Nearly every pixel-art human fighter
  on itch.io in a compatible look is a small side-scroller sprite, 25–45
  pixels tall, where our still battle pictures are 64–130. The bigger ones
  are either realistically proportioned or not pixel art.
- **Nothing matches Harl** (heavy, red beard, kettle helm, mail, axe).
  **No rider on a flying mount** exists in a compatible style (one
  riderless pegasus). **No old hooded priest with spectacles** exists as a
  fighter.
- Two sellers' licences forbid use in any project that also uses
  AI-generated code or writing (Mana Seed, Sleepless Seven). This game's
  code is written by Claude, so both are out.
- No ripped or edited Nintendo art was among the pages read.

**Shortlist** (best first within each gap; all allow commercial use and
edits and forbid reselling the files, unless noted):

| Gap | Pack, seller, price | What it has | Style | Catch |
| --- | ------------------- | ----------- | ----- | ----- |
| 1 map sprite | *Time Fantasy RPG Sprites 2*, finalbossblues, $10, <https://finalbossblues.itch.io/time-fantasy-monsters> | 8 horses and 24 characters with horse-riding versions, top-down, 4 directions | 4 | Characters are taller than our 16×20 (size not stated on the page). Licence is only the seller's replies in comments ("OK for commercial use… you can do pretty much anything other than redistribute the assets directly"). Buy on itch: the RPG Maker store copy is RPG Maker-only |
| 1 map sprite | *Time Fantasy Mini Sprites*, finalbossblues, free, <https://finalbossblues.itch.io/tf-mini-sprites> | 120+ characters, each frame "fits into a single 16x16 tile space"; a horses sheet | 4 | Whether riders are mounted on the horses isn't shown |
| 1 map sprite | *Tiny RPG Character Asset Pack 01*, Zerie, name your price, <https://zerie.itch.io/tiny-rpg-character-asset-pack> | 22 animated characters incl. a Lancer on horseback, Knight, Archer, Priest | 4 | Side view only, 24–32 pixels |
| 1 combat picture | *Mounted Pixel Art Knight Character*, GandalfHardcore, $5.19 on sale ($7.99), <https://gandalfhardcore.itch.io/mounted-knight-pixel-art-character> | A plate-armoured lancer on a caparisoned horse, 128×111 frames, 8 colours, one 64×64 portrait | 3 | The right size and true pixel art, but realistic proportions next to our chibi fighters |
| 1 both | *Heroic Asset Series: Creature Pack* ($8) and *Hero Pack* ($7), Aleksandr Makarov, <https://iknowkingrabbit.itch.io/heroic-creature-pack> | A mounted cavalier with lance, pikeman, archer, monk, griffin; portraits for every unit; 12 mounted heroes | 3 | About 16 pixels and cartoony; credit required; the seller says his payouts are frozen |
| 2 | *Bandits Pack*, Dreamir, $5, <https://dreamir.itch.io/bandits-pack1> | Bandit (bald, axe), Marauder (big, large axe), hooded Assassin; animated, side view | 4 | 40–45 pixels tall: under our smallest battle picture |
| 2 | *Side-View Animated RPG Battlers*, finalbossblues, $7.99, <https://finalbossblues.itch.io/side-view-animated-rpg-battlers> | 80 small fighters incl. a red-haired armoured one with a double axe | 4 | Small |
| 2 | *Librarium Statics: RPG Bandits I*, Aekashics, $5, <https://aekashics.itch.io/librarium-statics-rpg-bandits-i> | Three large bandits, one a bruiser with a long axe; map sprites | 2 | Painted, not pixel art; credit required; full terms page wouldn't open |
| 3 | *8-Bit Heroes 2: Archer1*, Sagak Art, $3, <https://sagak-art-pururu.itch.io/8bitheroes-archer1> | A male Robin Hood-style archer in a feathered cap, animated | 4 | About 35–45 pixels; the closest look to Tiny Tales found |
| 3, 4 | *10 Human Warriors JRPG battlers pack*, Fat Cat Games, $4.99, <https://fatcatgamesstudio.itch.io/10-human-warriors-jrpg-battlers> | **Still** battle pictures, 3 colours each: Archer, Cleric (hooded, face hidden), Druid (old, bearded), Halberdier, Paladin, Swordsman, Warlord, Wizard, Amazon, Valkyrie | 3 | The only true still battlers found; grittier and more dithered than Tiny Tales; sizes not stated; no faces. The free edition is non-commercial: buy the paid one |
| 3 | *Bandits Pack 2*, Dreamir, $5, <https://dreamir.itch.io/bandits-pack-2> | A male archer (no hood), a spearman | 4 | Small, as Bandits Pack |
| 5 | *8-Bit Heroes 2: Fighter*, Sagak Art, $3, <https://sagak-art-pururu.itch.io/8bitheroes-fighter> | A shirtless male martial artist, 7 motions | 4 | Small |
| 6 | *Time Fantasy* set: Sprites 1 ($15), Faces ($8, 104 faces) | Soldiers, knights, heroes and villains, with faces | 4 sprites, 3 faces | Faces are more cartoon than our anime ones |

**Rejected, with the reason** (so nobody rechecks them): Mana Seed *Hardy
Horse* ($19.99) and Sleepless Seven portraits (licence forbids projects
using AI-made code); *PIXEL BATTLERS 150+*, *400+ Pixel Art RPG Character
Sprites*, hsdsz's bandit packs, *GBA Style: Bandit Portraits* (AI-made or
AI-assisted, and the last imitates Fire Emblem's portraits); Low / Robert
Pinero's Archer, Cleric and Monk sets (hooded archers and clerics with
faces, but smooth high-resolution art, not pixel art); Clockwork Raven's
battlers (realistic, muted); Komodo's *Pixel Pack 3* (RPG Maker only);
Elthen's Barbarian, Bandit King and Monk (tiny, and the licence page
wouldn't open); several single horse-and-rider sprites (crude or
platformer-sized). Not opened: Thalzon's free battlers (Dropbox archives),
Cyangmou.

**What that means for the decision.** No pack fills a gap the way the
Heroes packs do (face, big still picture and map sprite in one style). The
honest options per gap are: a **small animated sprite drawn at double
size** (Dreamir's Marauder for the brigands, Sagak's archer and fighter:
right look, chunkier pixels than our other fighters); a **true still
picture in a rougher style** (Fat Cat's archer, cleric and halberdier;
GandalfHardcore's mounted knight); **keep the stand-in**; or **rewrite the
character to fit art we own** (for example Aske as the elf archer the
picture shows). The mockups must show the double-size and rougher-style
candidates next to a Tiny Tales hero so Nick can judge the clash himself.

## Nick input

**Decision** (`ask-nick`). For each gap, show Nick the shortlisted packs
**rendered next to Tiny Tales art** on our screens (the store preview in
the combat scene beside the lead, on the battle map beside bought map
sprites, and in the dialogue screen if the pack has faces), at most three
gaps per message. For each pack state the price today, the licence in one
line, whether AI was used, and say honestly if the style clashes. Options
for every gap include **"keep the stand-in"** and, for a named character,
**"rewrite their written look to fit art we have"** (Nick allowed that for
Rue).

## Scope

**In:** the search (done above; redo it if more than a month old, since
prices and catalogues change), the mockups, the decision recorded in
`docs/design/look-and-feel.md`; after Nick buys, a purchase record in
`THIRD_PARTY_ASSETS.md` as for the Mega Tiles bundle, and updates to the
tickets that use the art (0035 for Harl, 0413, 0436, 0706; later chapters'
content tickets).

**Out (do not do):** buying anything (Nick does); commissions; drawing or
redrawing art; anything ripped or edited from a commercial game (for
example real Fire Emblem sprites); packs under copyleft or non-commercial
licences (ADR-0013, ADR-0032); art that is AI output with no human touch.

## Implementation steps

1. Re-open each shortlisted store page: confirm the price and read the
   licence text again on the day (ADR-0032 rule 1). Drop anything that
   changed for the worse.
2. Make the mockups from store previews, in the scratchpad, never
   committed (as 0021 did).
3. Ask Nick, gap by gap. Record his words and picks in `look-and-feel.md`
   (*Portraits and battle art*).
4. For each pack he buys: add the purchase record, put the files with the
   other bought art (0110), and update the tickets named in *Scope*. If
   0706 gave a character a stand-in generator face in the meantime, swap
   it.
5. If a gap stays open (nothing fits and Nick keeps the stand-in), write
   that down with the stand-in he named, so later tickets don't ask again.

## Acceptance criteria

- [ ] Every gap has Nick's answer recorded: a pack, a stand-in, or a
      rewritten look.
- [ ] Each chosen pack's licence text, price and date are recorded before
      Nick buys.
- [ ] 0035, 0413, 0436 and 0706 match the answers.
- [ ] `cargo xtask ticket-lint` and `typos` pass.

## Tests required

- None (docs only).

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
