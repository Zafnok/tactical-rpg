---
id: "0504"
title: "Bot support in core: public legal-command list and luck reseeding for planning copies"
type: feature
milestone: M4 Enemy AI
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 0504 — Bot support in core: legal commands and luck reseeding

## Context

First technical step of the automated playtesting bots (Nick, 2026-09-30;
targets decided in 0033; runner 0505; bots 0506; trained bot 0507;
calibration 0508). A bot, like AlphaZero on Go, is given the rules (every
move it may make and whether it won) but not a strategy. It plans by trying
moves on copies of the battle. `core` is already built for that
(ADR-0004: pure, deterministic; `BattleState` is `Clone`). Two pieces are
missing:

1. **The list of every legal move.** It exists, but only inside the tests:
   `legal_commands` and its helpers (`legal_moves_after`, `legal_talks`,
   `legal_item_uses`, `legal_casts`, `legal_shop_txns`,
   `legal_skill_commands`, `legal_art_commands`) in
   `crates/core/src/battle/tests.rs` (≈ lines 2568–2960), used by the random-
   play property tests there and in `crates/core/src/history/tests.rs`. They
   `unwrap()` table lookups and document test-only assumptions ("Consumables
   in test packs are all known").
2. **Hidden luck.** The battle's `SimRng` lives inside `BattleState` so that
   a rewind repeats the same results (`docs/design/death-and-difficulty.md`,
   `crates/core/src/history.rs`, test
   `doing_the_same_thing_after_a_rewind_gives_the_same_result`). A bot that
   copies the state and tries an attack therefore learns the real outcome in
   advance: it would never attack into a miss. Planning copies need their
   luck replaced so the bot plans on the odds, like a human. **The real
   battle's luck is never touched**, so rewind behaves exactly as Nick
   designed (same attack after a rewind → same result).

Nick decided bots never use rewind (0033 context). Nothing here touches
`BattleHistory`.

## Nick input

None.

## Scope

**In:**
- A public `legal_commands(&BattleState) -> Vec<Command>` in `core`, correct
  for any loaded content, with its helpers.
- `BattleState::reseed_luck(&mut self, seed: u64)`.
- The existing property tests switched to the public function.
- An ADR for the bot architecture (see step 4).

**Out (do not do):**
- Any bot, search, runner or metrics (0505, 0506).
- Changing how the real game rolls luck or rewinds.
- Changing `ai::next_command` behaviour.
- A Python binding or anything for training (0507).

## Implementation steps

1. **New module `crates/core/src/legal.rs`** (re-exported from `lib.rs` as
   `trpg_core::legal_commands`). Move `legal_commands` and every helper it
   uses out of `battle/tests.rs`. Make it production code:
   - No `unwrap`/`expect`: a missing class, spell or item entry means that
     unit's option is skipped (the state would reject it anyway). Clippy's
     workspace lints must pass.
   - Remove the test-only assumptions: consumables come from the real
     `BattlePack` and `ItemTable`; shops enumerate what `legal_shop_txns`
     enumerates now, but document the limit (e.g. single-transaction visits
     only if that's what it does) in the doc comment.
   - Keep the order deterministic (same state → same list, same order); bots
     and tests rely on it. Document the order.
   - Module docs in the style of `ai.rs`: what's included (every command
     `apply` accepts in this state, grouped by kind), and the known
     exclusions if any, stated precisely.
2. **Tests keep working:** `battle/tests.rs` and `history/tests.rs` import
   `crate::legal_commands` instead. Delete the moved code from `tests.rs`.
3. **`BattleState::reseed_luck(&mut self, seed: u64)`** in `battle.rs`:
   replaces the state's `rng` with `SimRng::new(seed)`. Doc comment: for
   bots' planning copies only; the game never calls it; the real battle keeps
   its luck so rewinds repeat results (`death-and-difficulty.md`).
4. **ADR** (`write-adr` skill, next free number): "Playtest bots". Record:
   bots issue the same `Command`s as the player and read `legal_commands`;
   planning copies reseed luck and the real battle never does; bots never use
   rewind (Nick, 0033); bots and their runner live outside the shipped game
   (a `trpg-bots` crate created by 0505, used only by `xtask` and tests, never
   a dependency of `trpg-ui` or `trpg-app`); training tools (0507) are dev
   tools and are not shipped, so ADR-0013's shipped-dependency rules don't
   apply to them, but nothing trained ships without a new ADR.
   Add it to `docs/adr/README.md`.

## Acceptance criteria

- [ ] `trpg_core::legal_commands` is public and documented; no `unwrap` or
      `expect` in `legal.rs`.
- [ ] Property test `every_legal_command_is_accepted`: from random seeded
      setups (`arb_setup`), random play choosing only from `legal_commands`
      never gets a `CommandError`, to the end of the battle or 200 commands.
      (The existing random-play property in `battle/tests.rs` may be renamed
      to this.)
- [ ] Property test `ai_commands_are_legal`: for random states reached as
      above, `ai::next_command(state, &AiWeights::STARTING)`, when `Some`, is
      in `legal_commands(state)`.
- [ ] Unit test `legal_commands_is_deterministic`: two calls on equal states
      give equal lists.
- [ ] Unit test `reseeding_changes_luck_but_not_the_board`: two copies of a
      state, reseeded with different seeds, differ only in their RNG (the
      board, units and turn are equal); applying the same attack over a range
      of seeds gives both a hit and a miss for a ~50% attack; the same seed
      twice gives identical events.
- [ ] Unit test `the_real_battle_keeps_its_luck`: existing rewind test still
      passes unchanged (`doing_the_same_thing_after_a_rewind_gives_the_same_result`).
- [ ] ADR written and indexed.
- [ ] All gates in the `run-gates` skill pass, including the mutation gate's
      expectations for new `core` code.

## Tests required

- Unit: as listed.
- Property: the two properties above (ADR-0007 layer 2).
- Snapshot / integration: none.

## Completion notes

