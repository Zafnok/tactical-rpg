---
id: "0809"
title: "Play the victory sting, defeat sting and Game Over music"
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0022", "0023", "0024", "0214", "0801", "0807"]
nick_input: sign-off
completed:
---

# 0809 — Victory sting, defeat sting, Game Over music

## Context

Tickets 0022, 0023 and 0024 pick the music for three moments at the end of a
battle ([`docs/design/audio.md`](../../docs/design/audio.md), *Music cues*):

| Cue | Decided in | Plays |
| --- | ---------- | ----- |
| `victory` | 0022 | Once, while the `VICTORY` banner is shown |
| `defeat` | 0023 | Once, while the `DEFEAT` banner is shown |
| `game_over` | 0024 | On the Game Over screen (loop or once: see 0024's answer) |

The banners exist already (0405, `crates/ui/src/screens/battle/banner.rs`,
`BannerKind::Outcome`). The Game Over screen comes from 0801. Battle music
comes from 0807, which today makes Game Over **stop** the music; this ticket
replaces that with `game_over`. Audio plumbing: 0212 (ADR-0026), `ui` emits
cues through `ctx.audio` (`crates/ui/src/audio.rs`: `play_music`,
`stop_music`), only `app` plays them (ADR-0004). Tracks are imported the way
0214 did it (`assets-src/audio/import.py`, ADR-0027).

If one of 0022–0024 recorded "no music here" or "reuse `<cue>`", follow that
answer for that moment and say so in the PR.

## Nick input

**Sign-off:** win and lose a battle (Quick Battle or Chapter 1) and say
whether the stings and the Game Over music fit and are timed right.

## Scope

**In:**
- Import the chosen files with `import.py` (loudness-matched like the
  others), add them to `assets/audio/audio.ron` (`looped: false` for the two
  stings, and for `game_over` if 0024 chose "once"), `assets/audio/licenses`
  if needed, `THIRD_PARTY_ASSETS.md`, and the credits data.
- On `BattleEnded`, when the outcome banner appears: `play_music("victory")`
  or `play_music("defeat")`. The battle track fades out as usual (0.5 s,
  `music_fade_ms`).
- The Game Over screen emits `play_music("game_over")` when shown, replacing
  0807's `stop_music`.
- *Claude's starting rules* (tunable; list them in the PR for Nick):
  - After the victory sting ends, nothing plays until the next scene's
    `@music` (0710) or the next screen's cue. The sting is not cut short by
    the banner closing; a new cue fades it out as usual.
  - Rewinding (0307) can't happen after `BattleEnded`, so no rewind rule is
    needed. `Retry chapter` from Game Over starts the battle's own track again
    (0807).

**Out (do not do):**
- Level-up or EXP sounds (Nick's own work).
- World map, capital, camp and shop music (0025–0028, built by 1007, 1003,
  0409).
- A results screen (0810).

## Implementation steps

1. Import: add the three tracks to `assets-src/audio/import.py`'s list with
   the source URLs and versions recorded in `audio.md`, run it, commit the
   `.ogg` files and the updated manifest entries.
2. In the battle screen, where `BannerKind::Outcome` is created on
   `BattleEnded`, emit the matching music cue once.
3. In 0801's `GameOverScreen`, replace the `stop_music` from 0807 with
   `play_music("game_over")`.
4. Update `docs/design/audio.md` only if a starting rule above was changed
   by Nick at sign-off.

## Acceptance criteria

- [ ] Harness: winning a battle emits `play_music("victory")` exactly once,
      when the `VICTORY` banner appears.
- [ ] Harness: losing (the lord falls) emits `play_music("defeat")` once,
      then the Game Over screen emits `play_music("game_over")`.
- [ ] Harness: `Retry chapter` from Game Over emits the battle's own cue.
- [ ] The content validator loads the new cues; `looped` matches the rules
      above; each has a credit.
- [ ] `THIRD_PARTY_ASSETS.md` lists the new tracks.
- [ ] Nick signed off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: manifest entries (looped flags, credits) via the existing audio
  manifest tests.
- Snapshot / integration: the three Harness flows above, asserting on
  `AudioRequest`s.

## Completion notes

*(Filled in by the session that completes the ticket.)*
