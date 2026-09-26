---
id: "0413"
title: "Combat Arts in the attack flow: arts list, forecast with arts, durability display"
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0312", "0404", "0412"]
nick_input: sign-off
completed:
---

# 0413 — Combat Arts menu and forecast

## Context

0312 adds Combat Arts to `core` (`docs/design/combat-arts.md`). The player
needs to pick an art when attacking and see what it will do before
committing. 0412 already lets the player cycle combat actives on the
forecast. This ticket turns that into one list of arts **and** combat
actives, as in `combat-arts.md` → *Forecast display*. The UI only shows
`core`'s forecast; it never computes numbers
([ADR-0004](../../docs/adr/0004-crate-architecture.md)).

## Nick input

**Sign-off:** attack a few enemies with arts in Quick Battle. Is it quick to
pick an art, and can you tell what it will do and what it costs?

## Scope

**In:**
- An arts/actives list in the attack flow (0404), after choosing a target.
- The forecast panel with the chosen art applied, a line with its name and
  the durability change, and text notes for non-number effects.
- Durability `left/max` on the weapon line of the forecast and the unit info
  screen (0405).
- Debuff markers (Pinned, Slowed) on the map and info screen, reusing 0412's
  timed-effect markers.
- Enemy-phase playback shows the name of a boss's art or active.

**Out (do not do):**
- Core rules (0312).
- Boss AI (0503).
- The blacksmith/repair UI (0409).

## Implementation steps

1. After a target is chosen, show a list under the forecast:
   `Attack` (default), then usable arts, then combat actives. Each row shows
   name, cost (`−4 dur`, `+1 use`) and source (`E`/`D` rank, `weapon`,
   `active`). Arts and actives the unit knows but can't pay for (broken
   weapon, not enough durability) are shown dimmed with the reason. Arts
   that don't fit (e.g. Close Shot only matters with a bow) aren't listed.
   Moving the list cursor updates the forecast live. This replaces 0412's
   `s` cycle for combat actives.
2. The forecast panel adds a first line `‹Art name› (20 → 16)` when an art or
   active is chosen, and shows `core`'s notes as text: `no counter`,
   `pierces`, `pins: Mov −3`, `slows: Spd −3`, `stance: avoid +20`.
   Follow the mockup in `combat-arts.md`.
3. Weapon lines everywhere (forecast, weapon choice list, info screen) show
   durability `20/20`, and `broken` in the warning colour at 0.
4. Map/info markers for debuffs, with when they expire.
5. Enemy-phase playback: a one-line banner with the art or active name when
   a boss uses one (`ArtUsed`/`SkillUsed` events).
6. Help bar (0406) text for the arts list keys.

## Acceptance criteria

- [ ] Harness: a Swordsman picks Guard Break against a Brigand; the forecast shows `Guard Break (20 → 16)`, hit 100 and `no counter`, matching `core`'s forecast; after committing, the weapon shows `16/20`.
- [ ] Harness: with the weapon at 3/20, Guard Break is dimmed and can't be chosen.
- [ ] Snapshots: arts list, forecast with an art, forecast with a note-only art (Pinning Shot), broken weapon line.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Harness + snapshots as above.

## Completion notes
