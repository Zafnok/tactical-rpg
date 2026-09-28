---
id: "0307"
title: Turn rewind
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: medium
status: done
blocked_by: ["0006", "0305", "0404"]
nick_input: answer-first
completed: 2026-09-28
---

# 0307 — Turn rewind

## Context

Nick chose limited rewind charges in 0006 (FE Three Houses' Divine Pulse /
Echoes' Turnwheel); rules in `docs/design/death-and-difficulty.md`. Our
deterministic engine makes this cheap: store the initial state + command list
and replay up to any point.

## Nick input

**Answer first:** 0006.

## Scope

**In:** `core::history::BattleHistory`, rewind charges, a `RewindScreen` in `ui`,
a `Rewind` action + key binding (`r` in the right-handed layout, `u` in the left-handed one, per `docs/design/controls.md`).

**Out:** anything outside battles.

## Implementation steps

1. `BattleHistory { initial: BattleState, commands: Vec<Command>, charges_left: u8 }`;
   `push(cmd)`; `state_at(n) -> BattleState` by cloning `initial` and replaying
   `commands[..n]`; `rewind_to(n)` truncates commands and decrements a charge.
   Rewinding does not re-randomise: the RNG state is part of `BattleState`, so
   replaying gives identical outcomes — **document that rewinding + doing the
   same thing gives the same result** (like Turnwheel); doing something different
   changes the RNG consumption naturally.
2. Charges start at `BattleSetup.rewind_charges` (from the map's difficulty
   tier: easy 2, normal 3, hard 5, finale 8; 0801); a rewind costs one charge
   regardless of distance and may go back to any earlier command, enemy
   commands included. Charges don't carry between battles (unused ones become
   an EXP bonus, 0801); restarting a battle (0801) starts a fresh
   `BattleHistory` with full charges. Expose `charges_left()`.
3. `Action::Rewind` (in `trpg-content::keymap`, re-exported by `trpg-ui`) and its keys in every layout in `keymap.ron` (`r` right-handed, `u` left-handed).
4. `RewindScreen` (overlay): lists past actions newest-first as readable lines
   (`Turn 2 · Ana attacked Brigand (hit, 7 dmg)`), `j/k` to choose,
   the map behind shows the state at the highlighted point, `f` confirms (with a
   "Use 1 of N charges?" confirm), `d` cancels.
5. Integrate into the battle screen (0403/0404): `r` during player phase opens it.

## Acceptance criteria

- [x] Rewind restores exactly the prior state (test compares serialised states).
- [x] Charges enforced; zero charges → screen says so and can't confirm.
- [x] Harness test: attack, rewind, state equals pre-attack; snapshot of the rewind screen.

## Tests required

- Unit + property: for random valid command sequences and random `n`, `state_at(n)` equals the state obtained by applying the first `n` commands directly.
- Snapshot + Harness integration.

## Completion notes

- **Core:** `trpg_core::history::BattleHistory` (initial state, commands,
  charges left): `push`, `state_at(n)`, `replay()` (every command with the
  state before it and its events, in one pass), `rewind_to(n)` (one charge,
  any earlier point), `charges_left()`, and serde + `restore_tables` for the
  suspend save (0802). The module docs say rewinding and repeating the same
  action gives the same result, since the RNG is in `BattleState`.
  Unit tests plus a property test over random legal command sequences
  (`state_at(n)` and `rewind_to(n)` equal the directly played state,
  compared serialised).
- **Keys:** `Action::Rewind`, `r` right-handed / `u` left-handed; the
  layout picker's legend lists it.
- **UI:** `RewindScreen` in `screens/battle/rewind.rs`, drawn in the side
  panel: title, charges left, past actions newest first
  (`Turn 1 · Test Lord attacked Brigand (2 hits, 17 dmg)`, wrapped to the
  panel), the map showing the battle just before the highlighted action,
  Confirm → `Use 1 of N charges?` → Confirm rewinds, Cancel backs out.
  Harness tests: attack → rewind → state equals pre-attack (serialised);
  same attack again gives the same result; zero charges; snapshot.

**Deviations**

- The ticket said `j/k` to choose; the list uses the layout's up/down keys
  instead (arrows / `w s`), because `j`/`k` are Confirm/Cancel in the
  left-handed layout.
- `RewindScreen` is owned by the battle screen rather than pushed on the
  screen stack: it draws the battle map with another state and hands the
  chosen point back, which a stacked screen can't do.

**Claude's starting rules** (design docs silent; Nick can veto):

- Rewind opens only while browsing the map in the player phase (not while a
  unit is selected/moving/in a menu), and not once the battle has ended
  (game over offers Retry/Title per the design).
- Every command is a rewind point: each unit action, a unit's move after
  its attack, and phase ends. Picking the newest entry undoes just the last
  action.
- A rewind always asks for confirmation (`Use 1 of N charges?`).

Not in this ticket: the unused-charge EXP bonus and refund on restart
(0801), saving the history in the suspend save (0802), a hint for `r` in
the map's help line (0406). To try it: Quick Battle, attack with the lord,
press `r`.
