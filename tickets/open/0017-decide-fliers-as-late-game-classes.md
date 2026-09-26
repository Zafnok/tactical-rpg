---
id: "0017"
title: "Decide: make fliers a late-game (tier 3+) class line"
type: design-decision
milestone: M5 Progression
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 0017 — Decide: make fliers a late-game (tier 3+) class line

## Context

While revisiting terrain in ticket 0301 (`docs/design/terrain.md`), Nick said:
"Flyers I guess they'll be late game? Like tier 3+. We can adjust the class
progression to make that work." Today `docs/design/progression.md` has
**Flier** as a tier 1 class (Mov 7, flying) promoting to Sky Lancer / Sky
Warden (tier 2) and Storm Lancer / Sky Tyrant (tier 3). Fliers ignore most
terrain (cost 1 almost everywhere, 3 on peaks), which is why Nick wants them
late.

## Nick input

**Decision** (run with the `ask-nick` skill). Questions to ask:

1. How does a unit become a flier? Options to offer, with real-game examples:
   promotion branch at tier 3 only (e.g. a Rider or Lancer branch into a
   flying tier 3, like FE Three Houses' Wyvern Lord / Falcon Knight coming
   from advanced classes); a rare item or seal that turns a mounted unit into
   a flier (like FE Engage's / Fates' special seals); fliers only as late
   recruits who join already promoted (like FE GBA's late pegasus / wyvern
   joiners); or Nick's own.
2. What happens to the current tier 1 Flier, tier 2 Sky Lancer / Sky Warden
   and their skills (Swoop, Sky Dodge): removed, renamed, or kept as the
   tier 3+ versions?
3. Do enemy fliers also appear only late (no enemy fliers before the chapter
   where the player can get one)?

## Scope

**In:** the questions above; record the answers in `progression.md` (class
tables, tree diagram, skills), and edit any open ticket whose steps assume a
tier 1 Flier (search `tickets/open/` for "Flier" / "flying").

**Out (do not do):** code or data changes (class data is loaded by 0302 and
later tickets); terrain costs (decided in `terrain.md`).

## Implementation steps

1. Read `docs/design/progression.md` (tier tables, class tree, skills) and
   `docs/design/terrain.md`.
2. Ask Nick the questions with the `ask-nick` skill.
3. Update `progression.md` with Nick's words and the new rules; update the
   design README table if needed.
4. Update affected open tickets in the same PR.

## Acceptance criteria

- [ ] `progression.md` records Nick's words verbatim and the flier rules.
- [ ] No tier 1 or tier 2 flying class remains unless Nick chose that.
- [ ] Open tickets that referenced the old tier 1 Flier are updated.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None (design document only); `cargo xtask ticket-lint` passes.

## Completion notes
