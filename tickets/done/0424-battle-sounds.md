---
id: "0424"
title: "Battle sounds: strikes, spells, heals and movement"
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: done
blocked_by: ["0212", "0213", "0214"]
nick_input: sign-off
completed: 2026-09-29
---

# 0424 — Battle sounds

## Context

[`docs/design/audio.md`](../../docs/design/audio.md) ("Sound effects") lists
which sound plays for each battle event. Battle events are played back inside
the battle screen (ADR-0025, `crates/ui/src/screens/battle/playback.rs`).
This ticket makes playback emit the sound cues (0212's `AudioRequest`) at
the right moments.

## Nick input

**Sign-off:** play a battle (Quick Battle) with sound on and comment on how
each sound lands: timing, volume, and whether anything gets annoying.

## Scope

**In:** these rules, all from `audio.md`:

| Event | Cue |
| ----- | --- |
| A strike misses | `miss` |
| A weapon strike hits with damage > 0, no crit | `hit_sword` / `hit_spear` / `hit_axe` / `hit_bow` / `hit_gauntlet` by the striker's weapon kind |
| A weapon strike crits | `crit_physical` (instead of the hit sound) |
| A strike hits for **0 damage** (weapon or spell) | `block` (instead of the hit sound) |
| A spell goes off (`Event::SpellCast`) | `cast_fire` / `cast_ice` by element; nothing for `Element::None` |
| A spell strike hits | `hit_magic` |
| A spell strike crits | `crit_fire` / `crit_ice` by element; `Element::None` plays `hit_magic` (placeholder, `audio.md` open sub-question) |
| A unit is healed (`Event::Healed`) | `heal` |
| A unit moves (`Event::UnitMoved`) | one step sound per tile entered, by the class's `movement_type`: `foot` → `step_foot` (random variant each step), `armored` → `step_armored`, `mounted` → `step_mounted`, `flying` → nothing yet |

- Spells play in two beats: the cast sound when the spell goes off, then the
  hit or crit sound when the strike lands, timed to the existing playback
  animation.
- Both sides make the same sounds (Nick: "movement - on both sides"). Enemy
  moves come through the same playback path; if enemy-phase playback (0502)
  isn't done yet, its moves get sounds when it lands, with no extra work.
- Skipping playback (Cancel, ticket 0418) cancels any sounds not yet played.
  It doesn't cut off a sound already playing.

**Out (do not do):**
- Music (0807, 0814). Menu and cursor sounds (0425).
- Sounds for events `audio.md` doesn't list (level up, EXP, weapon rank up,
  terrain burning out, death). Don't invent them. If one feels badly missing,
  write a `00xx` follow-up question.
- An Absorb strike (the target heals from its own element) uses `hit_magic`
  like any other landing spell. This is *Claude's starting rule*: list it in
  the PR for Nick.

## Implementation steps

1. Find where playback steps through `Event`s and each strike in
   `CombatResolved`. Add a function `sound_for_strike(...) -> Option<&str>`
   and one for events. It needs the striker's weapon kind or spell element;
   find where the forecast/playback already has it (`CombatNumbers`,
   `Equipped`).
2. Emit `ctx.audio.play_sound(cue)` when each beat starts (the strike's
   impact frame, each movement tile).
3. Harness tests with a scripted combat per row of the table above, plus a
   move of each movement type.

## Acceptance criteria

- [x] A Harness test per row of the table: the right cue, once, at the right
      playback step.
- [x] A foot unit moving 5 tiles emits 5 `step_foot` requests.
- [x] Skipping playback emits no further sound requests (test).
- [ ] Nick signed off on the feel. *(Pending: play the PR build.)*
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `sound_for_strike` for every combination (hit/miss/crit/0 damage ×
  each weapon kind and element).
- Snapshot / integration: Harness playback tests above.

## Completion notes

- New `crates/ui/src/screens/battle/sounds.rs`: the cue rules as pure
  functions (`sound_for_strike`, `cast_sound`, `step_sound`), what each
  fighter strikes with (`combat_attacks`, following the command's
  `Equipped` events), and a small `CueQueue` for timed cues.
- `Playback::with_sounds` places the cues on the combat timeline: cast
  sound when the striker's name starts flashing, strike sound when the
  result shows. `Playback::sounds` hands out only the cues a frame
  reaches, so Cancel (skip) drops everything not yet played.
- Movement: the player's walk (`Mode::Moving`) plays a step per tile as
  the unit enters it (`Mode::tiles_entered`). A move with no walk shown
  (Canto's move-after now, enemy moves once 0502 lands) plays its steps
  from `Event::UnitMoved` at the walk's pace. The unit whose walk was
  shown doesn't step twice when its `Act` command applies.
- Deviation: step 1 said "one function for events"; heals and non-combat
  casts go through `event_cues`, while combat casts are timed by the
  playback instead (so they sit on the animation).
- `game.rs`'s title-music test now ignores sound requests (the battle it
  plays makes a hit sound now).

**Rules decided here** (Nick's answers after the first review: 1 vetoed,
3, 5 and 6 agreed; he'll judge 2 and 4 in the Quick Battle once it has a
caster, follow-up ticket 0428):
1. An Absorb strike (the target heals from its element) plays `heal`.
   Nick vetoed the starting rule (`hit_magic`): "it should sound like a
   heal". Recorded in `audio.md`.
2. Every spell strike plays its cast sound as the caster's name flashes,
   including a follow-up and a spell counter, not only the first cast.
3. A critical that does 0 damage plays `block`, like any 0-damage hit.
4. A heal during a combat (e.g. a draining skill) plays `heal` when the
   combat box's final hold starts; any other heal plays it at once.
5. A spell cast on a tile (fire on a forest, ice on water) plays its cast
   sound.
6. Steps come at the walk's pace (12 tiles/s); skipping a walk with
   Confirm skips its remaining steps.

