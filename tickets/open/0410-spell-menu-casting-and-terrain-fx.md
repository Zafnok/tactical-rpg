---
id: "0410"
title: "Spell menu, heal and tile targeting, affinity markers, terrain-change display"
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0309", "0310", "0404", "0407"]
nick_input: sign-off
completed:
---

# 0410 — Spell menu and casting UI

## Context

This ticket shows the magic from `docs/design/magic.md` (0004) on the battle
screen. It sends the `Cast`/`Equip` commands from 0309 and 0310 and reacts to
`SpellCast`, `Healed`, `TerrainChanged` and `SpellUsesChanged` events
([ADR-0004](../../docs/adr/0004-crate-architecture.md)). It follows the menu
patterns of 0404 (attack, forecast) and 0407 (item, equip).

## Nick input

**Sign-off:** in Quick Battle, cast Fire on a forest and Frost on water, heal
an ally, and hit an elemental. Comment on readability.

## Scope

**In:**
- `Magic` entry in the action menu, the spell list with `uses_left/uses`.
- Attack-spell targeting via 0404's forecast.
- Heal targeting with an HP preview.
- Tile targeting for Fire/Frost.
- Affinity markers in the forecast and the info screen.
- Drawing burning/burnt/ice tiles and a short change animation.
- Help-bar text.

**Out (do not do):**
- Battle notes (0411).
- AI playback (0502).
- New spells or rules.

## Implementation steps

1. Action menu: `Magic` is enabled when the unit knows a spell with a legal
   target from `dest`. The spell list shows `Fire  6/10  Mt5 Hit90 Rng1-2`,
   with spells at 0 uses dimmed.
2. Attack spell → 0404's targeting and forecast. The forecast shows `!` for
   Weak, `(resist)` for Resist, and `heals N` for Absorb. The equipped marker
   in 0407's `Equip` menu also lists attack spells.
3. Heal → target mode over valid allies with a preview (`HP 10 → 25`), and
   the 0407 `+N` popup on `Healed`.
4. Tile cast → the cursor snaps between valid tiles (0310's rules) and a
   preview names the change (`Forest → Burning (1 round)`, `Sea → Ice`).
   Confirm sends `Cast { Tile }`.
5. On `TerrainChanged`: redraw the tile with a 0.4 s flash (timing tunable).
   The info screen (0405) lists the unit's spells with uses, and its
   affinities.
6. Help bar: `f cast · d back` in the spell modes.

## Acceptance criteria

- [ ] Harness: Magic → Fire → forest tile → the tile shows burning; after End Turn and the enemy phase, it shows burnt.
- [ ] Harness: Magic → Heal → adjacent ally → HP rises by the design amount; the uses counter drops.
- [ ] Snapshots: the spell list, a forecast with each affinity marker, the tile preview, burning/burnt/ice tiles.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Harness and snapshot as above; cancelling at each level leaves the state unchanged.

## Completion notes
