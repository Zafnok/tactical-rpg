---
id: "0822"
title: "Game Over offers Rewind while charges are left"
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: sign-off
completed:
---

# 0822 — Game Over offers Rewind while charges are left

## Context

Nick (ticket 0408, 2026-10-01), asked where a defeat should land in a
battle with Preparations: "back on preparations, with another option to
Rewind if charges are left". `Retry Battle` going back to Preparations is
done (0408). This ticket is the other half: a defeat can be undone with a
rewind charge instead of starting the battle over
(`docs/design/death-and-difficulty.md`, *Rewind* and the game-over rules).
Nick wants it in before the Chapter 1 playtest (PR #140: "yes"), so 0804
is blocked by this ticket.

Today a lost battle closes the battle screen and the flow shows Game Over
with `Retry Battle` and `Title`; the battle and its rewind history are
dropped (`crates/ui/src/flow.rs`, `FlowScreen::battle_over`). The rewind
screen itself exists (0307: `crates/ui/src/screens/battle/rewind.rs`,
`RewindScreen`; the battle screen's `can_open_rewind` refuses once the
battle has an outcome).

## Nick input

**Sign-off:** Nick loses a battle with charges left, rewinds from Game Over
and plays on; then loses with none left and sees no `Rewind`.

## Scope

**In:**
- `Rewind` as Game Over's first item when the lost battle has at least one
  rewind charge left; hidden (not dimmed) otherwise.
- Choosing it returns to the battle with the rewind screen open; picking a
  point spends a charge as any rewind does and play carries on from there.
  Backing out of the rewind screen without picking returns to Game Over.
- Works the same in Classic and Casual, with and without Preparations.

**Out (do not do):**
- Changing what a rewind costs or how far back it goes.
- Game Over music and stings (0809).
- Saving (0802).

## Implementation steps

1. `crates/ui/src/screens/game_over.rs`: `GameOverChoice::Rewind`;
   `GameOverScreen::new(can_rewind: bool)` puts `Rewind` first when true
   (focused), then `Retry Battle`, `Title`.
2. `crates/ui/src/flow.rs`: keep the lost `BattleScreen` when showing Game
   Over (e.g. in `Stage::GameOver`), and pass
   `battle.history().charges_left() > 0`. On `Rewind`, put the battle back
   as the stage with its rewind screen open.
3. `crates/ui/src/screens/battle/mod.rs`: a way to open the rewind screen
   on a decided battle from the flow (e.g. `BattleScreen::rewind_from_defeat`),
   which also clears whatever the defeat left pending (banner, playback) so
   the battle plays on after the rewind. If the player closes the rewind
   screen without rewinding, the battle screen pops again and the flow
   shows Game Over again.
4. Check `BattleHistory` allows a rewind after the outcome is set
   (`crates/core/src/history.rs`); if not, allow it in core with a test:
   rewinding to before the defeat gives an undecided battle with one
   charge fewer.
5. Help lines through `widgets::help` (`keyboard-input` skill).

## Acceptance criteria

- [ ] Harness: lose the test battle with charges left → Game Over lists `Rewind`, `Retry Battle`, `Title` → `Rewind` → pick an earlier point → the battle is on again, undecided, with one charge fewer.
- [ ] Harness: backing out of the rewind screen returns to Game Over.
- [ ] Harness: with no charges left, Game Over lists only `Retry Battle` and `Title`.
- [ ] `Retry Battle` still restarts (through Preparations when the battle has them) with every charge back.
- [ ] Snapshot of Game Over with `Rewind`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `GameOverScreen` with and without `Rewind`; core rewind after a defeat.
- Snapshot / integration: the Harness flows above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
