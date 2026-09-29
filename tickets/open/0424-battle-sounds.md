---
id: "0424"
title: "Battle sounds: strikes, spells, heals and movement"
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0212", "0213", "0214"]
nick_input: sign-off
completed:
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
- Music (0807). Menu and cursor sounds (0425).
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

- [ ] A Harness test per row of the table: the right cue, once, at the right
      playback step.
- [ ] A foot unit moving 5 tiles emits 5 `step_foot` requests.
- [ ] Skipping playback emits no further sound requests (test).
- [ ] Nick signed off on the feel.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `sound_for_strike` for every combination (hit/miss/crit/0 damage ×
  each weapon kind and element).
- Snapshot / integration: Harness playback tests above.

## Completion notes

*(Filled in by the session that completes the ticket.)*
