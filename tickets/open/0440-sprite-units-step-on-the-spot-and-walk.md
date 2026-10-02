---
id: "0440"
title: "Sprite units step on the spot while waiting and walk along their path"
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0436"]
nick_input: sign-off
completed:
---

# 0440 — Sprite units step on the spot and walk along their path

## Context

Nick decided on 2026-10-02 (ticket 0039, `docs/design/look-and-feel.md`,
*Battle map: bought tiles and unit sprites*) that sprite units move like
Fire Emblem's on the GBA. He judged three animated mockups (J1–J3, then
J4 with the feet above the HP bar; in the bought-art folder's
`spike-renders/`) and picked "1C":

- A unit that **can still act steps on the spot**, facing the camera.
- A unit that **has acted stands still** (and is grey, 0436).
- A **moving unit walks along its path**: it glides from tile to tile
  with its legs going and turns to face the way it walks. When it arrives
  it faces the camera again.

He also said his Chapter 1 playtest **waits for this** ("B"), so 0804 has
this ticket in `blocked_by` and it is on the critical path in
`docs/ROADMAP.md`.

0436 draws each unit as the standing, front-facing frame of its bought
map sprite. Every sheet is 48×80: 3 columns (walking frames; the middle
one is the standing pose) × 4 rows (facing down, left, right, up) of
16×20 frames. Today a moving unit jumps one tile per step
(`ActionPlayback::walker_pos` in `crates/ui/src/screens/battle/
ai_phase.rs`, and the player's own move).

ADR-0038: what is shown goes in the `MapScene`; each map skin paints it.
The walking look is the sprite skin's; the glyph skin keeps today's look.

## Nick input

**Sign-off:** Nick sees it in his Chapter 1 playtest (0804), and before
that in a short screen recording or a few frames of the Quick Battle sent
to him (never committed: they show bought art). What to comment on: the
stepping speed, the walking speed, whether the map feels too busy. It
doesn't block the PR.

## Scope

**In:**
- The `MapScene` says, per unit: which way it faces, which walking frame
  it shows, and how far between two tiles it is.
- The sprite skins (0433's and 0436's mixed skin) paint that.
- Timing values in one place, *tunable*: a step on the spot every 250 ms
  (the frames go left foot, standing, right foot, standing); a walking
  unit crosses a tile in 200 ms and changes frame every 100 ms. These are
  the mockup's values. The animation-speed setting that already speeds up
  movement applies to walking too.

**Out (do not do):**
- Any change to the glyph skin's look: initials don't turn or step, and
  they keep jumping tile by tile. Every glyph snapshot stays unchanged.
- Any change to `core`, the bots or play records (ADR-0038): facing and
  frames are look only, and a unit's facing is never a rule.
- Attack or hit animations on the map; the combat scene (0413).
- Animated terrain (0437).
- Zoom (0439): **either order works.** Everything here is in tile
  fractions, so it follows whatever `tile_px` is.

## Implementation steps

1. **Scene** (`crates/ui/src/map_view/`, the `UnitView` from 0432):
   add `facing: Facing` (`Down`, `Left`, `Right`, `Up`; `Down` when not
   walking), `frame: u8` (0, 1 or 2; 1 is standing) and
   `offset: (f32, f32)` (how far toward the next tile, in tiles, each in
   `-1.0..=1.0`; `(0.0, 0.0)` when not walking). Add them to the scene's
   snapshot line only when they aren't the defaults, so existing scene
   snapshots don't change for still units.
2. **Who steps:** in `BattleScreen::scene`, a unit that hasn't acted,
   isn't fading and isn't walking gets `frame` from the screen's
   animation clock (the one the cursor pulse uses): the cycle 0, 1, 2, 1,
   one entry per 250 ms. An acted unit keeps `frame: 1`. All stepping
   units step together (one clock), as in the mockup.
3. **Walking:** where a walk is shown (`ActionPlayback` for the enemy
   phase and the player's own move), keep the tile-by-tile `pos` the
   rules and the camera use, and add the fraction of the way to the next
   tile as `offset`, the direction of that step as `facing`, and a
   `frame` that changes every 100 ms. The step sounds (0424) stay on the
   tile boundaries, where they are today.
4. **Sprite skin:** the unit's source frame is `(frame, row of facing)`
   in its sheet instead of 0436's fixed `(1, 0)`; `dest` moves by
   `offset × tile_px`, rounded to whole pixels. The outline, the HP bar
   and the effect arrows move with it. 0433's generated test tileset has
   one picture per unit: a tileset entry without walking frames ignores
   `facing` and `frame` and only glides. Say in the tileset format
   (`assets/tilesets/README.md`) how an entry declares that it has the
   3×4 layout.
5. **Glyph skin:** reads none of the three fields.
6. Render or record the Quick Battle with the bought sprites and **look
   at it**: the feet don't slide oddly, a unit turns at each corner of
   its path, nothing flickers when a walk ends. Send it to Nick.
7. Update `crates/ui/README.md` and `look-and-feel.md` if a value
   changed.

## Acceptance criteria

- [ ] Scene test: after a move that goes right and then up, the walking
      unit's `facing` was `Right` and then `Up`, its `offset` went from 0
      toward 1 on each step, and it ends with `Down`, `frame: 1`,
      `offset: (0, 0)`.
- [ ] Scene test: a unit that can act changes `frame` over time; a unit
      that has acted keeps `frame: 1`.
- [ ] 0433's `the_skin_never_changes_the_game` and
      `every_scene_feature_is_painted` cover the three new fields under
      both sprite skins.
- [ ] Every glyph-skin snapshot is unchanged.
- [ ] `git status` shows no bought file; the PR has no picture made from
      one.
- [ ] Nick was sent the recording or frames.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the frame cycle against the clock; the source rectangle for each
  facing and frame in a 48×80 sheet; `dest` for an offset; an entry
  without walking frames.
- Property: for any path, `offset` stays within one tile of `pos` and the
  unit ends on the path's last tile with no offset.
- Snapshot / integration: a scripted move in the Quick Battle under the
  sprite skin on the public fixture, read from the `MapScene`
  (ADR-0038), not from sprites.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
