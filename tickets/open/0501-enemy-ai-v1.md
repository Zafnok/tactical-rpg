---
id: "0501"
title: "Enemy AI v1: behaviours, target scoring, deterministic planning"
type: feature
milestone: M4 Enemy AI
model: opus-5.5
effort: high
status: todo
blocked_by: ["0305", "0306", "0309"]
nick_input: none
completed:
---

# 0501 — Enemy AI v1

## Context

Enemies must feel like Fire Emblem enemies: dangerous, predictable enough to
plan around, not stupid. The AI is pure `core` code issuing the same
`Command`s the player does ([ADR-0004](../../docs/adr/0004-crate-architecture.md)).

## Nick input

None (difficulty feel is judged in playtest 0804).

## Scope

**In:** `core::ai`, AI behaviour field on units, `assets/data/ai.ron` weights,
`next_command(state) -> Option<Command>` for the acting AI phase. Per
`docs/design/turn-structure.md` the AI runs **both** the Enemy phase and the
Other phase (`Ally` + `Neutral` units): the same code, with targets = units
hostile to the acting unit (`Faction::is_hostile_to`). Units that arrived as
reinforcements this turn are already acted and must be skipped.

**Out:** UI playback (0502), fancy group tactics, difficulty modes.

## Implementation steps

1. `AiBehavior` enum on `Unit` (set by chapter data): `Aggressive` (charge the
   nearest target), `Guard` (attack only if a target is in its threat area this
   turn, else hold), `Stationary` (never move; attack only from its tile —
   bosses on forts), `Healer` (heal spells from `magic.md`/0309: heal the
   most injured ally in reach, else keep distance behind allies). Any unit
   carrying its own consumable (0306) uses it on itself instead of acting
   when below 40% HP and no attack scores a kill (*tunable*).
2. **Attack choice:** for each stoppable dest × each target in range × each
   usable weapon or attack spell with uses left (0309; tile casts are not
   used by AI v1), score with `forecast`:
   `score = w_dmg * expected_damage + w_kill * P(kill) + w_lord * is_lord(target)
   - w_risk * expected_counter_damage + w_terrain * terrain_bonus(dest)`, where
   expected values use the displayed hit% (and doubling). Weights in `ai.ron`
   (tunable). Pick max; tie-break by `(target id, dest.y, dest.x, weapon id)`.
3. **Approach:** if no attack, `Aggressive` units move to the stoppable tile
   minimising distance (flow-field Dijkstra from all hostile units, using the
   mover's movement costs and ignoring units) to the nearest target; then `Wait`.
4. **Order within the phase:** units that can attack this turn first (highest
   score first), then movers ordered by distance to the nearest target, then id.
   `next_command` re-plans against the current state each call.
5. Every emitted command must be valid: `debug_assert!` + tests.

## Acceptance criteria

- [ ] Scenario tests (ASCII maps in tests): takes a guaranteed kill over a chip hit; prefers the lord when equal; `Guard` holds when nothing is in reach and attacks when something is; `Stationary` never moves; won't walk into impassable/hostile tiles.
- [ ] Property: across random battles, every AI command passes `BattleState::apply` validation.
- [ ] Soak: AI-vs-AI (a temporary player-side AI using the same code) plays 100 seeded battles on `test_small.map` to completion or turn 50 without panics.
- [ ] Planning a 20-unit enemy phase on a 30×30 map < 50 ms in release (measured, noted).

## Tests required

As in acceptance criteria. Deterministic: same state → same command (test).

## Completion notes

