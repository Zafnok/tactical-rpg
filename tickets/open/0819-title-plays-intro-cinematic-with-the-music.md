---
id: "0819"
title: "Title screen plays the intro cinematic in time with the title music"
type: feature
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0036", "0226", "0227", "0811", "0817"]
nick_input: answer-first
completed:
---

# 0819 — Title screen plays the intro cinematic with the music

## Context

Nick (ticket 0036): the title's intro cinematic "should match the length of
the title song, and loop when the song does (i.e. take a brief pause same
as the song currently does) and then it can show our logo whatever it might
be and pause there during the brief pause before looping the song/cinematic
again".

The parts exist by now: the cinematic player draws any moment `t` of a
cinematic file (0817); `ctx.music_clock` says how far into its track the
music is (0227); the title has its logo (0811) and the `Press any key or
button` prompt on every build (0226). This ticket joins them in
`crates/ui/src/screens/title.rs`.

How the cinematic and the menu share the screen, what a press does, and
what happens when the player comes back to the title from the game are
Nick's answers to 0036's Q1 (`docs/design/title-screen.md`, *Intro
cinematic*). Build exactly those rules; this ticket doesn't restate them
because they aren't decided as it is written.

The title song (`title`, `music/new_sunrise_v1.ogg`) is 133.7 s; its last
note ends at about 117 s and the rest is silence until it loops.

## Nick input

**Answer first:** ticket 0036.

## Scope

**In:**
- `TitleScreen` plays the cinematic named by a constant (`"title"`),
  with `t` taken from the music clock, so picture and song stay together
  and loop together.
- The menu, the prompt and key presses behave as 0036 decided.
- A fallback so the cinematic still runs when there is no music clock.
- Until 0820 lands, `assets/cinematics/title.ron` is a stand-in: one map
  pan over the test battle, then the logo from 117 s.

**Out (do not do):**
- The real shots and their timing (0820).
- New shot kinds (0818, 1010).
- An Options setting to turn the cinematic off: only if 0036 said yes, and
  then in 0805 (0036 has updated that ticket).
- Changing how the music starts, fades or loops (ADR-0026).

## Implementation steps

1. `TitleScreen` owns a `CinematicPlayer` for `title` (built on first
   update, since it needs `Ctx`). `draw` calls `player.draw(ctx, t, buf)`
   and then draws the menu / prompt over it as 0036 decided.
2. **Where `t` comes from**, each update:
   - `ctx.music_clock` is `Some` and its cue is `TITLE_MUSIC`: `t =
     position`. Never smooth or predict it; a one-frame lag is fine.
   - Otherwise (the track is still loading, its file is missing, or
     another track is fading out): hold `t = 0` for up to 3 s after the
     title asked for its music, then run `t` on the frame clock (`dt`),
     wrapping at the cue's `length_ms`. If the music clock appears later,
     switch to it.
   - Before the first press (the prompt is showing, 0226) there is no
     music yet: show what 0036 says the prompt screen shows.
3. Coming back to the title (a screen above it popped): the existing
   `music_on` flag asks for the title music again; it restarts from 0, and
   `t` follows the clock, unless 0036 said otherwise for this case.
4. Input: follow 0036's rules and the `keyboard-input` skill. The press
   that ends the prompt still does nothing else (`title-screen.md`). Any
   new text that names a key goes through the keymap.
5. The logo shot must be the same picture as the title's own logo (0817
   shares the drawing), so when the menu appears on it nothing jumps.
6. Update `docs/design/title-screen.md` only if a starting rule had to be
   added while building (mark it as Claude's, for Nick to veto).

## Acceptance criteria

- [ ] Harness: after the first press, the snapshot at music position 10 s
      is the stand-in's pan and at 120 s is the logo; after `wait`ing past
      133.7 s it shows the start again (it looped with the song).
- [ ] Harness with `music_load_delay(2.0)`: the picture holds at `t = 0`
      until the music starts, then follows it.
- [ ] Harness with `without_music()`: after 3 s the cinematic runs anyway
      and still loops at the track's length.
- [ ] Harness: each of 0036's Q1 rules has a test (what a press does, when
      the menu shows, returning from a battle).
- [ ] The existing title tests (menu choices, music requests, the prompt)
      still pass, changed only where 0036's rules require it.
- [ ] `screens_cover_the_whole_buffer` still holds for the title.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the choice of `t` (music clock, hold, fallback, switch back).
- Snapshot / integration: the Harness tests above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
