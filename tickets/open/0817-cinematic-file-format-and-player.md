---
id: "0817"
title: "Cinematic file format and player: timed shots, map pans, the logo"
type: feature
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: high
status: todo
blocked_by: ["0036", "0227", "0228", "0801", "0811"]
nick_input: answer-first
completed:
---

# 0817 — Cinematic file format and player

## Context

Nick wants an intro cinematic on the title screen (ticket 0036,
`docs/design/title-screen.md`, *Intro cinematic*): shots of a battlefield,
the overworld, characters and conversation snippets, as long as the title
song, ending on the logo. This ticket builds the machinery with the two
simplest shots (a map pan and the logo) and a test cinematic. Later
tickets add the other shots (0818), put it on the title in time with the
music (0819) and make the real one (0820).

Content is data (ADR-0005): the cinematic is a file listing shots and when
each starts, so 0820 and 1010 change it without code.

Builds on: 0227 (each music cue's length), 0228 (the pan and zoom window),
0801 (battle files: a map with its units), 0811 (the logo art and its
loader).

## Nick input

**Answer first:** ticket 0036 (answered 2026-10-03,
`docs/design/title-screen.md`, *Intro cinematic*): zoom is a whole step
(2×, 3×, 4×) and a shot keeps one size; shots change with **hard cuts**,
no fades between shots.

## Scope

**In:**
- The file format `assets/cinematics/*.ron`, its loader and validator in
  `trpg_content`, documented in `assets/cinematics/README.md`.
- `trpg_ui::cinematic`: a timeline and a player that draws the shot for any
  time `t`.
- Shot kinds `MapPan` and `Logo`.
- Hard cuts between shots (0036). The player still takes a whole-frame
  `brightness` (0..1) so the title can fade the cinematic in under its
  menu (0819); the file has no fades.
- `assets/cinematics/test.ron` and a debug tool to watch it.
- An ADR (`write-adr`) for the format and the "pure function of time" rule.

**Out (do not do):**
- Character and conversation shots (0818), the overworld shot (1010).
- Anything on the title screen, or following the music (0819). The debug
  tool runs on its own frame clock.
- The real title cinematic (0820).
- Units moving or fighting inside a shot: ticket 0825 (0036 chose it).

## Implementation steps

1. **ADR and format.** Recommended:
   ```ron
   (
     id: "test",
     music: "title",          // the cue whose length is the cinematic's length (0227's length_ms)
     shots: [
       (at: 0.0,  shot: MapPan(battle: "test", from: (4.0, 9.0), to: (12.0, 3.0), zoom: 2)),
       (at: 6.0,  shot: Logo),
     ],
   )
   ```
   `at` is seconds from the start. A shot lasts until the next shot's `at`;
   the last one lasts until the track's end. `from`/`to` are the map tile
   (fractions allowed) at the centre of the screen at the shot's start and
   end. `zoom` is 1, 2, 3 or 4 (0036: whole steps, one size per shot).
   There is no fade setting: shots cut. The length is the music cue's
   `length_ms` (0227); the file doesn't repeat it.
2. **Loader and validator** (`crates/content/src/cinematic.rs`,
   `Content::cinematics`, the all-assets test): shots sorted by `at`, the
   first at `0.0`, every `at` before the end; the battle exists; `from` and
   `to` are on its map; `zoom` is allowed; the music cue exists. All errors
   at once, with the file name.
3. **Timeline** (`crates/ui/src/cinematic.rs`): `Timeline::at(t) ->
   ShotAt { index, progress /* 0..1 */ }`, with
   `t` wrapped at the length. **Every shot is drawn from `t` alone**: no
   state carried from frame to frame. That is what lets 0819 follow the
   music, jump and loop, and lets tests snapshot any moment.
4. **Player**: `CinematicPlayer::new(ctx, id)` builds what the shots need
   once (each battle's scene: draw the whole map and its units at their
   starting places into one `GlyphBuffer`; move `BattleScreen::draw_terrain`'s
   body into a function both can call, and use `units::draw_unit`). If the
   map skin (0432, ADR-0038) is done, build a `MapScene` of the whole map
   instead and paint it with `ctx.map_skin`, so the cinematic follows
   whatever skin the battle uses; if it isn't, do as written here (0432
   then moves this code behind the skin).
   `CinematicPlayer::draw(&self, ctx, t, buf)` draws the current shot:
   - `MapPan`: the scene as a 0228 backdrop over the whole console, centre
     moving from `from` to `to` with an ease-in-out on `progress`.
   - `Logo`: 0811's title art, drawn by the same function `TitleScreen`
     uses (make it shareable; don't copy it).
   - `draw` takes a `brightness` (0..1) for the whole frame, used by 0819
     when the title's menu fades into the cinematic: `buf.dim` and the
     backdrop's brightness. It must cover everything in the frame: cells,
     rectangles and sprite items (ADR-0038). `dim` only changes cells, so
     fade items too (rectangles by colour, sprites by `opacity`).
5. **Debug tool** (`crates/ui/src/debug.rs`, `TOOLS`): "Play test
   cinematic", a screen that advances `t` by `dt` and loops; Cancel closes
   it. Help line per the `keyboard-input` skill.
6. `assets/cinematics/test.ron` on the test battle from 0801
   (`assets/battles/test.ron`), short (about 10 s).

## Acceptance criteria

- [ ] Content validation loads `test.ron`; each validator error has a test.
- [ ] Unit: `Timeline::at` at a shot's start, middle and end and at the
      wrap. A snapshot at `brightness` 0.5 dims cells and items alike.
- [ ] Snapshot: the test cinematic at three times (start of the pan, its
      middle, the logo). Drawing the same `t` twice gives the same buffer.
- [ ] Harness: F2 → "Play test cinematic" plays, loops and closes on
      Cancel.
- [ ] The logo is drawn by the same code as the title screen's.
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: loader/validator, `Timeline`.
- Property: for any `t`, `Timeline::at` returns a valid shot with
  `progress` in `0..=1`; `at(t)` equals `at(t + length)`.
- Snapshot / integration: the snapshots and the Harness test above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
