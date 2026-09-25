---
id: "0412"
title: Skill display, active-skill menus and forecast markers
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0311", "0404", "0405"]
nick_input: sign-off
completed:
---

# 0412 — Skill display and active-skill menus

## Context

0311 adds class skills to `core` (`docs/design/progression.md`, *Skills*):
always-on passives, and actives with uses per battle. The player needs to
see them and use them. This ticket covers the battle UI.

## Nick input

**Sign-off:** can you tell which skills a unit has, and is using an active
skill quick?

## Scope

**In:**
- A skills section on the unit info screen (0405), with names, rank,
  one-line effects and `uses_left/uses`.
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
   (`Unit::usable_skills()`), with `P`/`A` markers and uses for actives.
2. Attack flow: after choosing a target, `s` cycles through the usable
   combat actives (`none → Keen Edge → …`). The forecast updates, and a
   skill name line appears.
3. Action menu `Skill` → list of non-combat actives with uses. Picking one
   uses the same target cursor as items and heals.
4. A unit with a timed effect shows a small highlight in its glyph
   background (colour from the theme), and the info screen lists the effect
   and when it expires.

## Acceptance criteria

- [ ] Harness: use Brace from the menu, and the Def shown on the info screen rises. At the next own phase it is back to normal.
- [ ] Harness: choosing Keen Edge changes the forecast hit/crit to match `core`'s forecast.
- [ ] Snapshots of the skills block, the attack forecast with a skill, and the Skill menu.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Harness + snapshots as above.

## Completion notes
