---
id: "0412"
title: Skill display, active-skill menus and forecast markers
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: done
blocked_by: ["0311", "0404", "0405"]
nick_input: sign-off
completed: 2026-09-29
---

# 0412 — Skill display and active-skill menus

## Context

0311 adds class skills to `core` (`docs/design/progression.md`, *Skills*):
always-on passives, and actives paid with weapon durability (or an extra
spell use; `docs/design/combat-arts.md`). The player needs to
see them and use them. This ticket covers the battle UI.

## Nick input

**Sign-off:** can you tell which skills a unit has, and is using an active
skill quick?

## Scope

**In:**
- A skills section on the unit info screen (0405), with names, rank,
  one-line effects and each active's cost (`3 dur`, `+1 use`).
- In the attack flow (0404), an optional "use skill" choice that shows the
  forecast with the active applied.
- A `Skill` entry in the action menu (0403) for non-combat actives, with
  target selection.
- Timed-effect markers on the map and the info screen.

**Out (do not do):**
- Core rules (0311).
- Promotion screen (0603).

## Implementation steps

1. Unit info screen: a `Skills` block listing only the active ranks
   (`Unit::usable_skills()`), with `P`/`A` markers and the cost for actives.
2. Attack flow: after choosing a target, `s` cycles through the usable
   combat actives (`none → Keen Edge → …`). The forecast updates, and a
   skill name line appears with the durability change (`Keen Edge (20 → 17)`).
   Actives the unit can't pay for are skipped. 0414 later merges this
   cycle into one list with Combat Arts.
3. Action menu `Skill` → list of non-combat actives with their cost and the
   equipped weapon's durability; unaffordable ones dimmed. Picking one
   uses the same target cursor as items and heals.
4. A unit with a timed effect shows a small highlight in its glyph
   background (colour from the theme), and the info screen lists the effect
   and when it expires.

## Acceptance criteria

- [x] Harness: use Brace from the menu, and the Def shown on the info screen rises. At the next own phase it is back to normal.
- [x] Harness: choosing Keen Edge changes the forecast hit/crit to match `core`'s forecast.
- [x] Snapshots of the skills block, the attack forecast with a skill, and the Skill menu.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Harness + snapshots as above.

## Completion notes

- **Done.** New `battle::skills` module: skill text (cost `3 dur` /
  `+1 use`, one-line effects from the skill data), the combat actives the
  attack flow can cycle, the `Skill` menu and Shove's target mode.
  - Info screen: a `Skills` block (`A`/`P` marker, name, cost, effect line
    under it, highest rank per family), a `Effects` block right of the stats
    (name, changes, `until Player phase`), and stats that include timed
    effects, in the effect colour when changed.
  - Attack flow: in targeting, the Info key (`e`/`i`) cycles `none → Keen
    Edge → … → none`; the forecast is `core`'s with the active, and a line
    under the title reads `Keen Edge (20 → 17)`. Actives the weapon can't
    pay for are skipped; the help line names the key only when one exists.
  - Action menu: a `Skill` entry (between Seize and Item) for units that know
    a non-combat active; the list shows `Brace  3 dur  Wpn 20/20`, dimmed
    when unusable. Brace-like skills apply at once; Shove picks a target
    with the same cursor as items.
  - Map: a unit under a timed effect has the new palette colour `effect`
    behind its glyphs.
- **Deviations:** the ticket says `s` cycles the actives, but `s` is Next
  unit (right-handed) / Down (left-handed) in targeting, so the existing
  `Info` action does it (`e` / `i`), with no new key or keymap change.
  0414 replaces this cycle with its list.
- **Palette:** added the required colour `effect`.
- **Existing tests/snapshots updated on purpose:** the action menu now has
  `Skill` for units with a non-combat active (the lord's Inspire, the
  knight's Brace); the info screen's skills block; the archer's forecast
  help line names the skill key (Vault); the palette sampler.
- **Follow-up:** 0426 (actives that add range, such as Long Shot, don't yet
  offer their extra targets).
- **Claude's starting rules (please agree or veto):**
  - The skill-cycle key while targeting is the Info key.
  - The `Skill` entry only appears for units that know a non-combat
    active; it is dimmed if none can be used right now.
  - A skill that needs no target (Brace, Inspire, Sanctuary) is used the
    moment it is chosen in the list, with no extra confirm.
  - The effect marker is a purple-ish background behind the unit's two
    letters (same for buffs and debuffs).
