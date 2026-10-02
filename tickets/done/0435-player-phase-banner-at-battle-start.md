---
id: "0435"
title: "PLAYER PHASE banner at the start of a battle"
type: bug
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-02
---

# 0435 — PLAYER PHASE banner at the start of a battle

## Context

`docs/design/turn-structure.md` ("Exact sequence") says every phase start
shows its banner. Turn 1's Player phase doesn't: a battle opens straight on
the map, and the first banner the player sees is turn 1's `ENEMY PHASE`.

Why: `BattleState::new` returns the start events (with
`Event::PhaseStarted { turn: 1, phase: Player }`), but
`BattleScreen::start` (`crates/ui/src/screens/battle/mod.rs`) only queues
the scenes among them. 0405's completion notes left the banner to the game
flow (0801), which didn't add it. Found while working 0411, whose notes box
is meant to come "before the first `PLAYER PHASE` banner"
(`docs/design/magic.md`, "Battle notes").

Repro: New Game (or the debug Quick Battle) → the battle starts → no
`PLAYER PHASE` / `Turn 1` banner. Expected: the banner, as on turn 2.

## Nick input

`None.` (The design already asks for the banner.)

## Scope

**In:**
- The turn-1 `PLAYER PHASE` banner when a battle starts through
  `BattleScreen::start` (the game flow: New Game, Quick Battle, Retry,
  Restart Battle).
- Updating the tests that start a battle through the flow.

**Out (do not do):**
- Changing the banner's look, text or 1-second length (`banner.rs`).
- `BattleScreen::new` (a screen on a given state, used by unit tests and the
  debug tools): it stays without a banner.
- The Preparations screen (0408) and the chapter title card (0812).

## Implementation steps

1. Write the failing test first, in
   `crates/ui/src/screens/battle/turn_tests.rs`: a screen from
   `BattleScreen::start(state, &events)` (state and events from
   `BattleState::new`) has `banner()` of
   `BannerKind::Phase { phase: Phase::Player, turn: 1 }`; it closes on
   Confirm or after `PHASE_BANNER_S`; `BattleScreen::new` has none.
2. In `BattleScreen::start`, queue the start events the way
   `BattleScreen::apply` does: `Event::SceneTriggered` →
   `Queued::Scene`, anything else through `Banner::for_event` →
   `Queued::Banner`, in event order. This also shows the `VICTORY` /
   `DEFEAT` banner of a battle decided at its start.
3. Order: the battle notes (0411, `Queued::Notes`) stay first in the queue,
   then the banner and scenes in event order. Extend
   `notes_tests.rs` (`a_turn_one_scene_plays_after_the_notes` and a new
   test) to check notes → banner.
4. The start tip (0406, `TipTrigger::FirstBattleStart`) already waits for
   "a player phase browsing without a banner" (`shown_tip`): check it shows
   once the banner has closed (`tip_tests.rs`).
5. Fix the Harness tests that start a battle through the flow and press
   keys at once (`crates/ui/tests/battle.rs`, `controller.rs`, `flow.rs`,
   `split_keys.rs` if it does): while a banner is up only Confirm does
   anything, so their helpers (`quick_battle()`, `to_battle()`) must first
   close the banner (Confirm) or wait it out (`h.wait(1.0)`). Re-read every
   changed snapshot before accepting it (`run-gates` skill).

## Acceptance criteria

- [x] Harness: New Game → the test chapter's battle shows its notes box, then on Confirm the `PLAYER PHASE` / `Turn 1/3` banner, then the map.
- [x] Harness: the Quick Battle (no notes) opens on the `PLAYER PHASE` / `Turn 1` banner; it closes by itself after 1 second or on Confirm.
- [x] Retry and `Restart Battle` show the banner again.
- [x] `BattleScreen::new` shows no banner (test).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `turn_tests.rs`, `notes_tests.rs`, `tip_tests.rs` as above.
- Snapshot / integration: the Harness tests above; existing snapshots
  re-read where they change.

## Completion notes

**Done.** `BattleScreen::start` now queues the start events the way
`BattleScreen::apply` does: scenes as before, and everything else through
`Banner::for_event`. So a battle started by the game flow (New Game, Quick
Battle, Retry, Restart Battle) opens on `PLAYER PHASE` / `Turn 1`, after its
battle notes if it has any. `BattleScreen::new` is unchanged (no banner).

What the player sees at a battle's start, in order: the battle notes
(Confirm closes them) → the `PLAYER PHASE` banner (1 second, or Confirm) →
a turn-1 scene, if the battle has one → the first-battle tip, if tips are
on → the map. A battle already decided when it starts shows its `VICTORY` /
`DEFEAT` banner after the phase banner, instead of sitting on the map.

**No gameplay rules decided.** The order above is the order of the battle's
own start events, as the ticket asked.

**Tests.**
- Unit: `turn_tests.rs` (the banner on `start`, closing on Confirm or after
  `PHASE_BANNER_S`, none on `new`), `notes_tests.rs` (notes → banner → scene
  / tip; a decided battle's banners), `tip_tests.rs` (the start tip waits
  for the banner), `trigger_tests.rs` (a turn-1 scene plays after the
  banner).
- Harness: `battle.rs` has `quick_battle_opens_on_the_player_phase_banner`;
  `flow.rs` checks notes → banner → map on New Game, and the banner again
  after Retry and Restart Battle. The helpers that start a battle
  (`quick_battle()`, `to_battle()`, …) now close the banner with Confirm.
- Snapshots re-read: `battle__quick_battle_renders` and the four
  `controller__tip_*` changed only in the cursor's eight overlay lines (its
  colour a little further into its pulse, because the banner's frames pass
  before the picture is taken). Nothing else moved.

**Deviations.** `cancelling_the_menu_then_the_selection_restores_the_unit`
(`battle.rs`) compared the screen with the one "as the battle opened"; it
now takes its "before" picture after a select-and-cancel, which restarts
the cursor's pulse the same way.

**Follow-ups.** None.
