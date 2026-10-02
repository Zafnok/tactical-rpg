---
id: "0038"
title: "Decide: bought tiles and unit sprites on the battle map"
type: design-decision
milestone: M3 Battle UI
model: opus-5.5
effort: medium
status: done
blocked_by: ["0021"]
nick_input: decision
completed: 2026-10-02
---

# 0038 — Decide: bought tiles and unit sprites on the battle map

## Context

On 2026-10-02 Nick bought Mega Tiles' whole "2025 Bundle Sale" (37 products,
$99.99), not only the Heroes packs 0021 planned. Besides faces and still
battle pictures it holds 16×20 map sprites for every hero and battler class
and five 16×16 tilesets. Our map tile is 16×16 pixels (ADR-0018), so the art
fits the battle map with no scaling.

Nick asked for a spike: "what do you think about for instance, replacing our
in-battle (not in-combat) overhead units with the fixed smaller sprites?
Could we have a spike to see if that looks good? and what about the
tilesets, are any of those useful?" ADR-0038 left "which skin the game
ships with" to him.

## Nick input

**Decision**, made on rendered mockups of the battle screen (drawn with the
game's font atlas and palette at 2×, from the bought files; kept on Nick's
machine and never committed, ADR-0032):

- A: today's glyph terrain and name initials.
- B: glyph terrain, bought unit sprites.
- C: the bought World Map tileset (the artist's own sample map) with bought
  unit sprites. C2: the same at double size. C3: the same darkened toward
  palette D.
- D: four ways for a sprite unit to show its side (HP bar only, a coloured
  outline, a tinted tile, a corner mark).
- E: the Overworld and Dungeons tilesets with units standing in them.
- G1a/G1b, G2a/G2b: a combat picture with a bought battle background, in
  the box over the map and full screen.
- H: a 48×48 face at 5× beside an 80×80 bust at 3×.

## Scope

**In:** the decision, recorded in `docs/design/look-and-feel.md`; the
purchase recorded; the tickets that build it.

**Out (do not do):** building any of it; committing bought files or
pictures made from them.

## Implementation steps

1. Sort and index the bought files (outside the repository).
2. Render the mockups and ask Nick.
3. Record his answer; write the follow-up tickets; update the tickets it
   changes.

## Acceptance criteria

- [x] Nick saw the renders and his choice is recorded in
      `look-and-feel.md` with his words.
- [x] Follow-up tickets exist and the tickets it changes are updated.
- [x] `cargo xtask ticket-lint` and `typos` pass.

## Tests required

- None (docs only).

## Completion notes

**Decided (2026-10-02), in `look-and-feel.md` § *Battle map: bought tiles
and unit sprites* and § *Combat screen*:**

- The battle map is drawn with the bought tilesets and the bought unit map
  sprites. Nick: "I think we can go ahead and move forward with replacing
  our tile rendering and battler rendering with these bought sprites
  without regret".
- Lighting is set per map: low light is shaded like C3, noon is like C.
- A key and a button toggle the zoom between 1× and 2×, and the game
  remembers the choice for the next battle.
- In combat the two fighters always face each other; a picture that faces
  the wrong way is mirrored.
- Fighters with no fitting art get more bought packs in a similar style
  (Claude searches, Nick buys); characters who never fight may use the
  Character Generator.

**Answered the same day, after the PR opened:**

- A sprite unit's side is a 1-pixel outline in the side's colour, and an
  acted unit is grey and darker: "A, and potentially A with C" (C is a
  corner mark; 0436 renders the outline alone and with it, and Nick
  picks).
- The zoom key and button: "I think we can ask this as we build the zoom
  feature in another ticket? I will decide it later". 0439 asks it before
  building.
- The Chapter 1 playtest waits for the bought-art map ("A"): 0436 and
  0437 joined 0804's `blocked_by` and the critical path.

**Not decided, kept in 0039:** how a sprite unit shows an effect; whether
sprites walk; whether players may pick the glyph look; face or bust in
dialogue. Combat's layout (box or full screen) and background stay with
0413.

***Claude's starting rule:*** unit sprites keep their own colours in every
light (C3 was drawn that way and Nick didn't comment on it).

**Found on the real files** (corrects notes made from store previews):
faces and busts both come in 8 expressions; the 1× face sheet is 192×96
(the 576×288 one in 0711 is the 3× copy); the still battle pictures looked
at (heroes and the Vol.1, 4 and 5 classes) all face right as bought, so
whoever stands on the right must be mirrored; nothing in the bundle is
mounted; the only human with an axe is the Amazon Warrior hero.

**Follow-up tickets created:** 0039 (the open questions), 0040 (more packs
for fighters with no fitting art), 0436 (unit sprites), 0437 (tileset
terrain), 0438 (per-map lighting), 0439 (zoom toggle).

**Tickets changed:** 1006 (its question is answered; only the Claude-drawn
icons are left, and Nick didn't ask for them), 0433, 0413, 0706, 0711,
0110, 0035, 0029, 0037.

**Where the bought files are:** `D:\tactical-rpg\Tiny Tales Bundle Assets\`
on Nick's machine, ignored by git there (`.git/info/exclude`). Its
`index.html` shows every character's faces, battle picture and map sprite
together; `spike-renders/` holds the mockups.
