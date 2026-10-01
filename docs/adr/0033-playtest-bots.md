# ADR-0033: Playtest bots play through `core`'s commands, on reseeded copies, outside the game

- **Status:** Accepted
- **Date:** 2026-09-30
- **Related tickets:** 0033, 0504, 0505, 0506, 0507, 0508, 0509, 0510

## Context

Nick wants automated playtesting (`docs/design/playtest-bots.md`, ticket
0033): bots that play a battle as a Casual, Normal or Hardcore player, many
times, and report whether it is too hard or too easy. Like a game-playing
engine given only the rules, a bot is told every move it may make and whether
it won, not a strategy, and it plans by trying moves on copies of the battle.

`core` is already built for that (ADR-0004): pure and deterministic, changed
only by `Command` → `Event`s, and `BattleState` is `Clone`. Two things were
missing:

- **The list of legal moves** existed only in the tests, and only sampled
  arts and skills.
- **Hidden luck.** The battle's `SimRng` (ADR-0019) lives in `BattleState`,
  so that repeating an action after a rewind gives the same result
  (`docs/design/death-and-difficulty.md`). A bot that copies the state and
  tries an attack would learn the real outcome in advance and never attack
  into a miss; it would plan like a cheater, not a player.

Nick decided the bots never rewind (0033). The bots and their tools are dev
tooling: they must not grow the shipped game or its dependencies.

## Decision

1. **Bots issue the same `Command`s as the player**, through
   `BattleState::apply`, and choose from `trpg_core::legal_commands(&state)`
   (module `core::legal`): every command `apply` accepts in the state, in a
   documented deterministic order, built from candidates filtered by
   `BattleState::check` (the validation `apply` runs, without applying) so
   the rules stay in one place. The one documented cut: shop visits are
   listed as single transactions plus buying two of the shop's first item.
   A bot never reaches into the state any other way.
2. **Planning copies reseed their luck; the real battle never does.**
   `BattleState::reseed_luck(seed)` replaces a copy's `SimRng`. Bots call it
   on every copy they plan on, with seeds from their own RNG, so they plan
   on the odds. The game never calls it: the real battle keeps its luck and
   rewinds repeat results as Nick designed. Any other bot-only state change
   (e.g. hiding unarrived reinforcements from a copy, 0506) follows the same
   rule: a documented `core` method for copies only, never called by the
   game.
3. **Bots never use rewind** (Nick, 0033). The bots' driver never builds a
   `BattleHistory`.
4. **Bots live outside the shipped game.** They and their runner go in a
   `trpg-bots` crate (created by 0505, `publish = false`), pure like `core`
   (no I/O, no clock). It is used only by `xtask` and tests and is never a
   dependency of `trpg-ui` or `trpg-app`.
5. **Training tools are dev tools.** Anything for a trained bot (0507, e.g. a
   Python binding or ML libraries) is not shipped, so ADR-0013's
   shipped-dependency licence rules don't apply to it (its licence must
   still let us use it privately). Nothing trained — model files, weights,
   inference code — ships in the game without a new ADR.

## Consequences

- Bots are as strong as their search, never stronger than the rules: a bot
  can't make a move a player couldn't, or know luck a player couldn't.
- `legal_commands` must keep up with the rules. Because it filters through
  `check`, a new refusal is picked up automatically; a new kind of action,
  or anything that lets an attack reach farther, must be added to
  `core::legal` too. Its property tests (every listed command is accepted;
  the list equals a far wider search's accepted commands; the AI's commands
  are listed) catch a gap.
- `apply` is now "validate, then carry out" in two steps (`check` shares
  the validation), which also gives the UI a way to ask whether a command
  would be accepted.
- The bots' crate and tools add nothing to the shipped binaries or their
  licences.

## Alternatives considered

- **Keep the legal-move list in the tests; bots write their own** — two
  lists of the rules would drift, and a bot using a wrong one tests the
  wrong game.
- **Generate each kind of move exactly, without `check`** (the old test
  helper) — duplicates every rule in the generator; a missed rule lists a
  move `apply` refuses.
- **Hide luck by reseeding the real battle, or rolling luck outside the
  state** — breaks Nick's rewind rule (same action after a rewind, same
  result) and saves that repeat.
- **Let bots see the real luck** — they would never attack into a miss, so
  their results would say battles are easier than they are.
- **Bots inside `core` or the game** — ships dev code and, for trained bots,
  possibly unvetted dependencies.
