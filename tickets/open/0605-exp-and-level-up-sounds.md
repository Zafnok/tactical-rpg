---
id: "0605"
title: "Play the EXP bar and level-up sounds"
type: feature
milestone: M5 Progression
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0022", "0602", "0212"]
nick_input: sign-off
completed:
---

# 0605 — Play the EXP bar and level-up sounds

## Context

Ticket 0602 built the EXP bar, the level-up page and the class-progress box
(`crates/ui/src/screens/battle/progress.rs`, driven from
`BattleScreen::update` in `crates/ui/src/screens/battle/mod.rs`) with no
sound. Ticket 0022 records in
[`docs/design/audio.md`](../../docs/design/audio.md) which cue plays at each
moment. Ticket 0212 made the cue system: screens call
`ctx.audio.play_sound(cue)` (`crates/ui/src/audio.rs`), `app` plays it, and
every cue is listed in `assets/audio/audio.ron` (ADR-0026).

## Nick input

**Sign-off:** level a unit up in the Quick Battle (F2 debug) and listen. Do
the sounds make the level up feel exciting? Too loud, too long, tiring
after many level ups?

## Scope

**In:**
- Each moment 0022 gave a cue: the EXP bar filling, the bar reaching 100,
  the level-up page opening, each stat row that grew appearing, class
  mastery / learned. Only the ones 0022 decided; "no sound" means nothing.
- New sound files (rendered with `cargo xtask` like 0213's in-house sounds,
  or the chosen library files with their license text and a
  `THIRD_PARTY_ASSETS.md` row) and their `audio.ron` entries.

**Out (do not do):**
- Any cue 0022 didn't pick. Combat sounds (0424), menu sounds (0425).
- Changing the pages' timings or layout (0602's, Nick signs those off there).

## Implementation steps

1. Add the cue files and `audio.ron` entries (check the manifest test passes).
2. In `progress.rs`, give `Progress` a way to say which cues start this
   frame: e.g. `fn cues(&self, before: f32, after: f32) -> Vec<&'static str>`
   comparing the page clock before and after a `tick`, with one-shot cues on
   a page's first frame (page opened), per stat row revealed
   (`stats_shown()` rising) and when `exp_shown()` crosses a multiple of 100.
   A Confirm press that jumps to the end must not fire every skipped stat
   tick at once: play at most one per frame.
3. In `BattleScreen::update`, after `progress.tick` and `progress.confirm`,
   call `ctx.audio.play_sound` for each cue.
4. Harness tests (`crates/ui/src/screens/battle/progress_tests.rs` has the
   99-EXP level-up setup) using `Harness::audio_requests`.

## Acceptance criteria

- [ ] A level up from 99 EXP requests exactly the cues 0022 chose, in order,
      each once (a stat tick once per stat that grew).
- [ ] An EXP gain without a level up requests only the bar's cue (if any).
- [ ] Skipping a page with Confirm doesn't burst several cues in one frame.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the cue timeline in `progress.rs`.
- Snapshot / integration: harness tests of the audio requests.

## Completion notes

