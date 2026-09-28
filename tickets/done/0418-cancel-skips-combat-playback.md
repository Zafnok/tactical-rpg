---
id: "0418"
title: Cancel skips a fight's playback; holding Confirm speeds it up
type: tuning
milestone: M3 Battle UI
model: sonnet-5
effort: low
status: done
blocked_by: ["0404"]
nick_input: sign-off
completed: 2026-09-28
---

# 0418 — Cancel skips a fight's playback; holding Confirm speeds it up

## Context

0404 built the combat playback (`crates/ui/src/screens/battle/playback.rs`,
ADR-0025) with both controls on Confirm: hold for ×4, and a tap (a press
released within 0.2 s) skips to the end. After playing it, Nick asked
(2026-09-28) to split them:

> "I don't think skipping and fast playback of a fight should be bound to
> same hotkey, probably use the cancel hotkey (i.e. D) to skip and the
> confirm hold key (F) to playback faster"

## Nick input

**Sign-off:** in Quick Battle, attack the brigand in the middle. Hold `f` to
speed the fight up, press `d` to skip it. A quick tap of `f` does nothing.

## Scope

**In:**
- Cancel (`D`/`K`, `Esc`) skips the rest of a fight's playback.
- Holding Confirm keeps the ×4 speed-up. A Confirm tap no longer skips.
- Help line `d skip · hold f fast`; record the rule in
  `docs/design/controls.md`.

**Out (do not do):**
- Changing the speed-up (×4) or any playback timing.
- Enemy-phase playback (0502): only its input note is updated.

## Implementation steps

1. `playback.rs`: drop the tap detection (`Timings::tap`, `Playback::press`
   and its `press` field); add `Playback::skip`; `tick` only speeds up while
   Confirm is held.
2. `mode.rs` (`step`, `Mode::Combat`): Cancel calls `skip`; every other
   action is ignored.
3. `mod.rs` (`help`): `cancel("skip")` instead of `confirm("skip")`.
4. Tests and the mid-strike snapshot's help line.
5. `docs/design/controls.md`: Nick's words and the rule. Update 0502's
   input step to match.

## Acceptance criteria

- [x] During a fight, `d` skips to the end and the next keys work.
- [x] Holding `f` plays the fight ×4; a tap of `f` doesn't skip.
- [x] The help line reads `d skip · hold f fast`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `playback.rs` (hold, skip), `mode.rs` (Cancel skips, Confirm and
  other keys ignored).
- Harness: `cancel_skips_the_playback_and_the_next_keys_work`,
  `holding_confirm_plays_four_times_as_fast`, the Quick Battle fight in
  `tests/battle.rs`.

## Completion notes

- Done as listed. `Timings::tap` and `Playback::press` are gone; `skip()`
  jumps the clock to the end, and the same frame's tick returns to browsing,
  so no key after it is lost.
- Harness tests: `a_tap_skips_the_playback_and_the_next_keys_work` became
  `cancel_skips_the_playback_and_the_next_keys_work` and now also checks
  that tapping `f` leaves the fight playing. The Quick Battle test skips
  with `d`. Snapshot change: the mid-strike help line only.
- `docs/design/controls.md` records Nick's words and the rule (the Cancel
  row notes it too). ADR-0025's status line and index row note the key
  change (its decision, that playback input goes through the mode's `step`,
  still holds). 0502's step 2 now handles Cancel as well, so enemy fights
  skip the same way.
- No gameplay rules decided.
