---
id: "0505"
title: "cargo xtask playtest: run bots on a battle file and report the measures"
type: feature
milestone: M4 Enemy AI
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0504", "0801"]
nick_input: none
completed:
---

# 0505 — Playtest runner and battle measures

## Context

Second step of the automated playtesting bots (0033 targets, 0504 core
support, ADR written by 0504). This ticket builds the harness every bot runs
in: load a real battle file, play it many times with fresh luck, and report
the measures Nick described (did it win, units fallen, turns taken, items
used). It ships with one simple **baseline bot**: the enemy AI
(`ai::next_command`) playing the player's side, as the 0501 soak test
(`crates/content/tests/ai_soak.rs`) already does. The real persona bots come
in 0506 and plug into the same trait.

Needs 0801's battle files (`assets/battles/*.ron` and their loader) so it
plays real battles, not hand-built test setups.

## Nick input

None.

## Scope

**In:**
- New crate `crates/bots` (`trpg-bots`, `publish = false`): the `PlayerBot`
  trait, the baseline bot, `BattleMeasures`, and a `play_battle` driver.
- `cargo xtask playtest` subcommand.
- A speed figure (commands per second) in the report.

**Out (do not do):**
- Persona bots and target checks (0506).
- Rewind: bots never rewind (Nick, 0033). The driver never builds a
  `BattleHistory`.
- CI workflows (0506 decides with 0033's Q9 answer).
- Campaign-level play (between battles: shops, preparations, level choices).

## Implementation steps

1. **Crate `crates/bots`** (`trpg-bots`), added to the workspace, depending
   on `trpg-core` (and `trpg-content` only if the driver needs content types).
   It must never become a dependency of `trpg-ui` or `trpg-app` (ADR from
   0504). Pure like `core`: no I/O, no clock; the xtask does files and timing.
2. **`BattleMeasures`** (`bots/src/measures.rs`), filled by
   `observe(&mut self, state_after: &BattleState, events: &[Event])` after
   every applied command:
   - `outcome: Option<Outcome>` (from `Event::BattleEnded`), `None` if the
     turn cap was hit;
   - `turns: u32` (highest turn seen in `Event::PhaseStarted`);
   - `player_fallen: Vec<UnitId>` (`Event::UnitFell` for units whose faction
     in the state is `Faction::Player`; look them up in
     `state_after.fallen()`);
   - `items_used: BTreeMap<ItemId, u32>` and a total (`Event::ItemUsed` by
     player units);
   - `commands: u32` applied (player side and total).
3. **`PlayerBot` trait**: `fn choose(&mut self, state: &BattleState) ->
   Command` for the Player phase. Bots get a `seed: u64` at construction for
   any randomness of their own and for reseeding planning copies (0504).
   **`BaselineBot`**: `ai::next_command(state, weights)` or `EndPhase` when it
   returns `None`.
4. **`play_battle(state, bot, enemy_weights, turn_cap) -> BattleMeasures`**:
   loop until `state.outcome()` is `Some` or the turn cap passes; Player
   phase → the bot; other phases → `ai::next_command(state, enemy_weights)`,
   else `Command::EndPhase` (the game's rule, `ai.rs` module docs). A
   `CommandError` from the bot is a bug: return it as an error with the
   command and turn, don't skip it.
5. **`cargo xtask playtest <battle-id> [--runs 100] [--seed 1] [--mode
   classic|casual] [--turn-cap 60] [--bot baseline] [--json <path>]`**
   (`crates/xtask/src/playtest.rs`): load content and the battle via 0801's
   loader; run `runs` tries, try `i` with battle seed `seed + i` (fresh luck
   each try); time each run with `std::time::Instant`. Runs may use
   `std::thread::scope` across cores; results must not depend on the thread
   count (sort by try index before reporting).
6. **Report** (text to stdout; the same data as JSON with `--json`): battle,
   mode, bot, tries, wins / losses / turn-cap, win %, turns (median, min,
   max), player units fallen (mean, max), items used (mean, per item),
   commands per second. Example layout to keep:

   ```
   ch01 · Classic · baseline · 100 tries (seeds 1–100)
   won 63  lost 35  turn cap 2   (63%)
   turns     median 14  min 11  max 22
   fallen    mean 1.8   max 5
   items     mean 3.1   (vulnerary 2.9, antidote 0.2)
   speed     41 200 commands/s
   ```
7. Document the command in the xtask's `--help` and in a new
   `docs/playtesting.md` (what the bots are, how to run them, what each
   number means). Link it from `docs/ROADMAP.md`'s playtest-bots line.

## Acceptance criteria

- [ ] `cargo xtask playtest <a battle in assets/battles> --runs 20` prints the
      report above and exits 0.
- [ ] Same arguments twice → identical report except the speed line.
- [ ] `--runs 20` with 1 thread and with all threads → identical report
      except speed (test with an env var or flag for the thread count).
- [ ] Unit tests in `trpg-bots`: `measures_count_fallen_player_units_only`,
      `measures_count_items_used_by_player_units`,
      `measures_turns_is_the_last_turn_started`, each on a small scripted
      battle.
- [ ] Integration test `baseline_bot_plays_a_battle_to_the_end`: plays the
      shipped battle file (or `test_small` if 0801 provides one) for 5 seeds
      without a `CommandError`.
- [ ] `trpg-app` and `trpg-ui` don't depend on `trpg-bots`
      (`cargo tree -p trpg-app | grep trpg-bots` is empty; add this check to
      the test if cheap, else to `run-gates`).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: measures, as listed.
- Property: none new.
- Snapshot / integration: the baseline-bot battle test; determinism of the
  report.

## Completion notes

