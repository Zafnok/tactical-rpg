---
id: "0501"
title: "Enemy AI v1: behaviours, target scoring, deterministic planning"
type: feature
milestone: M4 Enemy AI
model: opus-5.5
effort: high
status: done
blocked_by: ["0305", "0306", "0309"]
nick_input: none
completed: 2026-09-28
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
reinforcements this turn are already acted and must be skipped. When the
battle has a `pending_move()` (a unit offered a move after its attack,
0311), `next_command` must answer it with `Command::MoveAfter` (stay, or
step to one of `move_after_tiles()`).

**Out:** UI playback (0502), fancy group tactics, difficulty modes, bosses
using Combat Arts and actives (0503; ordinary enemies never use them,
`docs/design/combat-arts.md`).

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

- [x] Scenario tests (ASCII maps in tests): takes a guaranteed kill over a chip hit; prefers the lord when equal; `Guard` holds when nothing is in reach and attacks when something is; `Stationary` never moves; won't walk into impassable/hostile tiles.
- [x] Property: across random battles, every AI command passes `BattleState::apply` validation.
- [x] Soak: AI-vs-AI (a temporary player-side AI using the same code) plays 100 seeded battles on `test_small.map` to completion or turn 50 without panics.
- [x] Planning a 20-unit enemy phase on a 30×30 map < 50 ms in release (measured, noted).

## Tests required

As in acceptance criteria. Deterministic: same state → same command (test).

## Completion notes

**What was built:**

- **`core::ai`** (`crates/core/src/ai.rs`): `AiBehavior` (`Aggressive`
  default, `Guard`, `Stationary`, `Healer`), a new `ai` field on `Unit`
  (serde default, so old saves load), `AiWeights`, and
  `next_command(state, weights) -> Option<Command>`. It plans afresh on
  every call and works for any phase. Every rule is in the module docs.
- **Attack scoring** uses the battle's own numbers: a new crate-internal
  `BattleState::plain_forecast` runs the same `fighters` + `forecast` code as
  an `Act`, skills and auras included, without re-checking the move. That
  keeps the search cheap. The expected values play out every hit/miss
  branch of the forecast's strikes, stopping when someone falls, as
  `resolve` does.
- **`assets/data/ai.ron`**, loaded by `trpg_content::ai` into
  `Content::ai`. Percentages over 100 are refused, with the line number.
- **`next_command` returns `None` when the AI has nothing left to do**
  (battle over, or no unit left to act), and the caller applies
  `EndPhase`. That is what 0502's step 1 already expects. A unit waiting
  to move after its attack gets its `MoveAfter` first.
- Tests (ADR-0007): 45 unit/scenario tests in `crates/core/src/ai/tests.rs`
  (ASCII maps), a proptest (random 9×7 battles, every faction and
  behaviour, spells, bows with Skirmish, potions; every command is accepted
  and repeats for the same state), the soak test
  `crates/content/tests/ai_soak.rs`, and `ai.ron` loader tests.

**Measured:** planning a whole 20-enemy phase against 20 players in contact
on a 30×30 map with forests and walls (20 `next_command` calls, applies
excluded) takes **4.5 ms in release** (budget 50 ms). Re-run with
`cargo test -p trpg-core --release -- --ignored enemy_phase_planning_time --nocapture`.
Soak: 100 seeded AI-vs-AI battles on `test_small.map` take 1.6 s in debug.
78 end in victory or defeat; the other 22 reach turn 50, typically when
both sides drew mostly Guard/Stationary units and nobody walks in. None
panicked, and every command was accepted.

**Deviations:**

- The `debug_assert!` on every emitted command applies it to a clone of the
  state (debug builds only), so it checks exactly what `apply` checks.
- The ticket's tie-break is `(target id, dest.y, dest.x, weapon id)`; a
  loadout slot is added last, for two copies of the same weapon.

**Claude's starting rules** (gameplay the design docs don't settle; all
*tunable*, judged in playtest 0804):

1. **Weights:** 10 per expected HP dealt, 300 × kill chance, +50 for
   attacking a lord, −5 per expected HP taken from the counter, +5 per point
   of the attacking tile's `Def + Avoid/10` (forest 3, fort and mountain 4
   and 5).
2. The AI reads **hit chances as displayed** (not the true two-dice odds)
   and **ignores crits** when weighing an attack.
3. **A unit that can attack always does**, even at a bad trade.
4. **Self-heal:** below 40% HP, a unit carrying a consumable drinks its
   first one on the spot, unless one of its attacks would kill with at
   least a 50% chance.
5. **Healers** heal the ally missing the most HP, standing outside the
   player's danger zone if they can. With nobody to heal they attack if
   they can reach someone; otherwise they move to the tile out of danger
   nearest an ally. Only `Healer` units heal; other units that know Heal
   don't.
6. **Guard** attacks anything it could reach this turn, otherwise it holds
   its tile. **Stationary** only attacks what it can hit without moving.
7. An **Aggressive** unit with no target it can walk to (e.g. across water)
   waits where it is.
8. **Moving after an attack** (Vault, Swoop, Skirmish): the AI steps to the
   tile farthest from the nearest hostile unit. It stays if no tile is
   farther.
9. **Order in a phase:** attackers first, best attack first; then everyone
   else, nearest to a target first.

**Follow-ups:** none. 0803 sets each enemy's behaviour in chapter data
(it already plans the Aggressive/Guard/Stationary mix). 0502 drives
`next_command` from the battle screen.
