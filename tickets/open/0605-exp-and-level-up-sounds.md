---
id: "0605"
title: "Play the EXP bar and level-up sounds"
type: feature
milestone: M5 Progression
model: sonnet-5
effort: medium
status: blocked
blocked_by: ["0602", "0212"]
nick_input: setup
completed:
---

# 0605 — Play the EXP bar and level-up sounds

## Context

Ticket 0602 built the EXP bar, the level-up page and the class-progress box
(`crates/ui/src/screens/battle/progress.rs`, driven from
`BattleScreen::update` in `crates/ui/src/screens/battle/mod.rs`) with no
sound. Nick is making the level-up and EXP sounds himself (recorded in
tickets 0022 and 0809: "Level-up or EXP sounds (Nick's own work)"). This
ticket plays them once he has supplied them. Ticket 0212 made the cue
system: screens call `ctx.audio.play_sound(cue)` (`crates/ui/src/audio.rs`),
`app` plays it, and every cue is listed in `assets/audio/audio.ron`
(ADR-0026).

The moments a sound can go with (see the 0602 snapshots in
`crates/ui/src/screens/battle/snapshots/*progress_tests*`):

1. The EXP bar filling (about 0.6 s).
2. The bar reaching 100 (a level up), just before the level-up page.
3. The level-up page opening (`LEVEL UP!`).
4. Each stat that grew appearing (one every 0.12 s).
5. `CLASS MASTERED!` / `Learned: <name>`.

## Nick input

**Setup:** Nick puts his sound files in the repo (or sends them) and says
which of the moments above each one is for; a moment without a file stays
silent. Blocked until then. Record his answer in `docs/design/audio.md`
(new cue rows; remove "level up" from the open sub-questions).

**Sign-off:** level a unit up in the Quick Battle (F2 debug) and listen:
too loud, too long, tiring after many level ups?

## Scope

**In:**
- The moments Nick gave a sound, and only those.
- Nick's files under `assets/audio/sfx/` with `audio.ron` entries. They're
  our own work (no `THIRD_PARTY_ASSETS.md` row); if any part isn't, check
  its license against ADR-0013 first.

**Out (do not do):**
- Picking or making sounds for moments Nick left silent. Combat sounds
  (0424), menu sounds (0425).
- Changing the pages' timings or layout (signed off in 0602).

## Implementation steps

1. Add Nick's files and their `audio.ron` entries (check the manifest test
   passes), and record the cues in `docs/design/audio.md`.
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

- [ ] A level up from 99 EXP requests exactly Nick's cues, in order,
      each once (a stat tick once per stat that grew).
- [ ] An EXP gain without a level up requests only the bar's cue (if any).
- [ ] Skipping a page with Confirm doesn't burst several cues in one frame.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the cue timeline in `progress.rs`.
- Snapshot / integration: harness tests of the audio requests.

## Completion notes

