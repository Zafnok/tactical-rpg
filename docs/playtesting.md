# Automated playtesting

Bots play a battle many times and a report says how it went: how often they
won, who fell and when, how many items they used, how many turns it took.
What the numbers should be (the bands per player type) is Nick's decision in
[`design/playtest-bots.md`](design/playtest-bots.md); how the bots are built
is [ADR-0033](adr/0033-playtest-bots.md).

## What a bot is

A bot is a player made of code. It plays the Player phase with the same
commands a person gives (move, attack, use an item, end the turn); the enemy
plays its own phases exactly as in the game. A bot:

- **never rewinds** and never restarts: one try is one play from the first
  turn;
- **can't see the dice**: it can't know whether an attack will hit before
  making it;
- knows nothing about how the game looks, so a change of graphics can't
  change a result.

A **try** is one play of a battle with fresh luck. Each try has a **seed**,
a number that fixes its luck: the same battle, bot and seed always play out
the same way, so any try in a report can be played again.

Bots so far:

| Bot | How it plays |
| --- | ------------ |
| `baseline` | The enemy AI playing the player's side: charges, attacks the best target, never uses items, never seizes or shops. A yardstick for the runner, not a real player. |

The Casual, Normal and Hardcore bots come with ticket 0506.

The army a bot fields is the characters of the battle's player slots, as
they are in the character data (the same army the debug Quick Battle uses).

## Running it

```bash
cargo xtask playtest quick
```

`quick` is a battle id: the name of a file in `assets/battles/` without
`.ron`. Options:

| Option | Default | What |
| ------ | ------- | ---- |
| `--runs <n>` | 100 | How many tries. |
| `--seed <n>` | 1 | Seed of the first try. The tries play seeds `n`, `n + 1`, … |
| `--mode classic\|casual` | classic | The game mode. |
| `--turn-cap <n>` | 60 | A try still going after this many turns is stopped and counted as "turn cap". |
| `--bot <bot>` | baseline | Who plays. |
| `--json <path>` | | Also write the whole report as JSON. |
| `--history <dir>` | `target/playtest-history/` | Where every run is kept. |
| `--threads <n>` | every core | Threads to play on. Changes the speed only, never the numbers. |

`cargo xtask playtest --help` prints the same list. A run is much faster
optimised: `cargo run --release -q -p xtask -- playtest quick`.

## Reading the report

```
ch01 · Classic · baseline · 100 tries (seeds 1–100)
won 63  lost 35  turn cap 2   (63%, was 58%)
turns     median 14 (was 15)  min 11  max 22
fallen    mean 1.8 (was 2.1)   max 5
items     mean 3.1 (was 3.0)   (vulnerary 2.9, antidote 0.2)
fallen/try 0: 55  1: 30  2: 12  3+: 3   most: Mira 22×, Kael 9×
speed     41 200 commands/s
previous  2026-09-30 08:00 UTC · commit fd6920d · 100 tries (seeds 1–100)

try 17 · lost (lord fell T9) · fell: Mira T6, Kael T8 · items 2 · turns 9
…
```

| Line | Meaning |
| ---- | ------- |
| first | Battle, mode, bot, and which seeds were played. |
| `won … lost … turn cap …` | How the tries ended, and the share won. |
| `turns` | Turns a try took: the middle try (median), the shortest and the longest. A try stopped by the turn cap counts as the cap. |
| `fallen` | Player units that fell per try (died in Classic, retreated in Casual): the average and the worst try. |
| `items` | Consumables the player's units used per try: the average, then the average per item. |
| `fallen/try` | How many tries had 0, 1, 2, and 3 or more units fall, and which units fell in the most tries. |
| `speed` | Commands played per second (every side's). The only line that differs between two runs with the same arguments. |
| `previous` | The run before this one, when there was one; its numbers are the `was …` next to the new ones. |
| `try N · …` | One line per try: `N` is its seed, then how it ended, who fell on which turn (`T6` = turn 6), items used, turns. |

To watch one try's numbers again: `cargo xtask playtest ch01 --seed 17
--runs 1`.

## History

Every run is added to a file in the history folder, one file per battle,
mode and bot (`quick-classic-baseline.jsonl`, one run per line, with the
date and the git commit). The next run of the same battle, mode and bot
prints the last one's headline numbers as `was …`, so a change to a battle
shows as "won 63% (was 58%)". The folder is under `target/`, so it isn't
committed and `cargo clean` empties it; pass `--history <dir>` to keep runs
somewhere else.

## For developers

- `crates/bots` (`trpg-bots`): the `PlayerBot` trait, `BaselineBot`,
  `play_battle` (one try) and `BattleMeasures` (what happened in it). Pure,
  like `core`: no files, no clock. It is dev tooling: `trpg-ui` and
  `trpg-app` never depend on it, and it never depends on them (a test in
  `xtask` checks the manifests).
- `crates/xtask/src/playtest.rs`: the command. It loads the battle, plays
  the tries on threads, times them, and writes the report, the JSON and the
  history.
- A new bot implements `PlayerBot` and is added to the `Bot` enum in
  `playtest.rs`. It gets the try's seed for its own randomness and for the
  copies it plans on (`BattleState::reseed_luck`); the same seed must give
  the same play.
