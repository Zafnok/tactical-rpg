---
id: "0035"
title: "Decide Harl's combat picture (Chapter 1 boss)"
type: design-decision
milestone: M3 Battle UI
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 0035 — Decide Harl's combat picture (Chapter 1 boss)

## Context

In 0021 (2026-09-30) Nick chose Mega Tiles' Tiny Tales packs for faces and
combat pictures (`docs/design/look-and-feel.md`, *Portraits and battle art
(bought)*; ADR-0032). The combat scene (0413) shows each fighter's big still
image, moved by the game. Named characters without their own art use their
class's picture, recoloured, and a character's face and combat picture must
match.

**Harl** (`docs/story/characters/red_captain.md`), the Chapter 1 boss, is a
**Brigand**: an enemy-only class that uses **axes only**
(`assets/data/classes.ron`). Weapon types have their own rules
(`docs/design/weapons-and-items.md`: axes always deal at least 5 damage but
hit less often; swords hit harder on follow-ups), so the picture's weapon
must match his. His brief: big, heavy-set, hunched like a bear, a battered
kettle helm, a heavy axe, a red beard, a rusted mail shirt under a red
Company jacket, aged 46.

Nothing in the Mega Tiles catalogue fits. Shown to Nick in 0021 and not
chosen:
- the *Vol.1 Monstrous Uprising* orc axe fighter, as sold and recoloured to
  human skin (Nick: "orc really seems crazy");
- the *Heroes 1* Dragon Knight (ornate dragon armour, a glaive-like pole
  weapon);
- the *Vol.4 Magitek Dynasty* dark knight (a sword, so it would change his
  class).

The *Gods and Gallants* bosses are gods, mechs and monsters. Nick vetoed
commissions ("expensive for just a single asset"). His words: "let's just
have a follow up for this Harl character I can shop around and find more
contenders".

## Nick input

**Decision.** Nick shops around (itch.io or elsewhere) and brings candidate
packs or links. For each, Claude checks and then renders:
- **Licence** against ADR-0032: commercial game OK, modification OK, no
  royalties, not AI output without a human touch.
- **Price:** Nick decides whether it's fine.
- **Style match:** a mockup of the combat scene next to a Tiny Tales hero
  (the lord), from the store preview scaled to our 2× pixel size, plus the
  dialogue screen with his face. Say honestly if the style clashes.
- **Weapon:** an axe keeps him a Brigand. Any other weapon means a class
  change for Harl, which Nick must confirm (it changes the boss fight and
  his character sheet).

Nick picks one, or keeps a stand-in for now.

## Scope

**In:** checking and mocking up the candidates Nick brings, and optionally
a short search by Claude for human axe fighters in a style close to Tiny
Tales. Record the choice in `look-and-feel.md` (*Combat screen*), and in
`red_captain.md` if his look or class changes.

**Out (do not do):** buying anything; commissions; drawing or redrawing
art (only the recolours and small edits `look-and-feel.md` allows);
implementing the combat scene (0413).

## Implementation steps

1. Collect Nick's candidates, plus up to three found by Claude (say which
   are which).
2. For each, record the store, licence text, price, whether it's
   AI-assisted, and the weapon.
3. Render the mockups (as in 0021: the store preview in our frames, in the
   scratchpad, never committed) and send them.
4. Record Nick's pick and his words. If his weapon or class changes, update
   `red_captain.md` and flag 0803 (Chapter 1 units).

## Acceptance criteria

- [ ] Nick saw a rendered mockup of every candidate and picked one (or
      chose to keep a stand-in).
- [ ] `look-and-feel.md` names Harl's picture and its source.
- [ ] `cargo xtask ticket-lint` and `typos` pass.

## Tests required

- None (docs only).

## Completion notes
