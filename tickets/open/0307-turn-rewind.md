---
id: "0307"
title: Turn rewind (if Nick chose it in 0006)
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0006", "0305", "0404"]
nick_input: answer-first
completed:
---

# 0307 — Turn rewind

## Context

Ticket 0006 recommends limited rewind charges (FE Three Houses' Divine Pulse /
Echoes' Turnwheel). Our deterministic engine makes this cheap: store the
initial state + command list and replay up to any point.

**First step: read `docs/design/death-and-difficulty.md`.** If Nick chose *no
rewind*, set `status: done`, write "Not needed: Nick chose no rewind in 0006"
in Completion notes, archive the ticket, and open a PR with just that change.

## Nick input

**Answer first:** 0006.

## Scope

**In:** `core::history::BattleHistory`, rewind charges, a `RewindScreen` in `ui`,
a `Rewind` action + key binding (`r` by default, added to `keymap.ron`).

**Out:** anything outside battles.

## Implementation steps

1. `BattleHistory { initial: BattleState, commands: Vec<Command>, charges_left: u8 }`;
   `push(cmd)`; `state_at(n) -> BattleState` by cloning `initial` and replaying
   `commands[..n]`; `rewind_to(n)` truncates commands and decrements a charge.
   Rewinding does not re-randomise: the RNG state is part of `BattleState`, so
   replaying gives identical outcomes — **document that rewinding + doing the
   same thing gives the same result** (like Turnwheel); doing something different
   changes the RNG consumption naturally.
2. Charges per design; a rewind costs one charge regardless of distance.
3. `Action::Rewind` in `trpg-ui` and `"r"` in `keymap.ron`.
4. `RewindScreen` (overlay): lists past actions newest-first as readable lines
   (`Turn 2 · Ana attacked Brigand (hit, 7 dmg)`), `j/k` to choose,
   the map behind shows the state at the highlighted point, `f` confirms (with a
   "Use 1 of N charges?" confirm), `d` cancels.
5. Integrate into the battle screen (0403/0404): `r` during player phase opens it.

## Acceptance criteria

- [ ] Rewind restores exactly the prior state (test compares serialised states).
- [ ] Charges enforced; zero charges → screen says so and can't confirm.
- [ ] Harness test: attack, rewind, state equals pre-attack; snapshot of the rewind screen.

## Tests required

- Unit + property: for random valid command sequences and random `n`, `state_at(n)` equals the state obtained by applying the first `n` commands directly.
- Snapshot + Harness integration.

## Completion notes

