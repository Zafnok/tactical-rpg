---
id: "0820"
title: "The title cinematic itself: shots and snippets timed to the title song"
type: content
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: high
status: todo
blocked_by: ["0036", "0706", "0707", "0803", "0818", "0819"]
nick_input: sign-off
completed:
---

# 0820 — The title cinematic itself

## Context

Everything needed to play a cinematic on the title exists (0817, 0818,
0819), but `assets/cinematics/title.ron` is a stand-in. This ticket makes
the real one from the storyboard Nick approved in 0036
(`docs/design/title-screen.md`, *Intro cinematic*): a pan/zoom over a
battlefield, characters, brief conversation snippets with no spoilers, and
the logo during the song's pause.

The title song (`title`, New Sunrise V1) is 133.7 s:

| Time | What the music does |
| ---- | ------------------- |
| 0:00–0:30 | Very quiet opening, slowly rising |
| 0:31–1:24 | Middle section, moderate |
| 1:25–1:56 | Full and loudest |
| 1:57–2:14 | The last note ends, then silence until the loop: **the logo** |

Real content it draws on: the Chapter 1 battle and map (0803), the cast's
bought faces (0706) and the Chapter 1 script (0707).

**No overworld shot yet** unless 0036's Q4 chose a cinematic-only map (then
0036 wrote a ticket for that map and added it to this ticket's blockers).
Otherwise 1010 adds the shot once the world map exists.

## Nick input

**Sign-off** after merge, on the Pages build and the Windows download:
watch the title through one whole loop with sound. Does each shot land
where the music changes, is the logo up for the whole pause, do the
snippets give anything away, does anything look off? Comments become
follow-up tickets.

## Scope

**In:**
- `assets/cinematics/title.ron`: the storyboard's shots at their times.
- `assets/dialogue/title_cinematic.dlg`: the snippet scenes, using only
  the lines and characters Nick approved in 0036.
- Small timing changes so cuts land on the music (keep within a second or
  two of the storyboard; anything bigger goes back to Nick).

**Out (do not do):**
- New shots, lines or characters that aren't in the storyboard. Ask Nick
  (`ask-nick`) if the storyboard can't be built as written.
- Code changes to the player or the title screen. If a shot needs
  something they can't do, write a ticket (`write-ticket`).
- Re-cutting the song.

## Implementation steps

1. Read the storyboard and its rules in `title-screen.md`. Listen to the
   song and note the exact second of each change you want a cut on.
2. Snippets (`story-writing` skill): copy the approved lines as they stand
   in the final Chapter 1 script (0707) into `title_cinematic.dlg`, one
   scene per snippet. If a line's wording changed since 0036, use the
   script's wording. If a line was cut from the script, or now reveals
   more than it did, stop and ask Nick for a replacement. Each text box
   must fit one page. No `@choice`. Don't update the ledger: nothing new
   is established.
3. Write `title.ron`: each shot's `at`, the map pans' start and end tiles
   and zoom on the Chapter 1 battle (`assets/battles/ch01.ron`), the
   characters, the snippets, and `Logo` at the song's last note
   (about 117 s; set it by ear).
4. Watch it with the sound on in the native build and the web build, all
   the way round the loop, at least twice. Fix overlaps, cut-off text and
   shots that feel too short to read (a snippet must stay up long enough
   to read twice).
5. Snapshot each shot at one moment.

## Acceptance criteria

- [ ] Content validation loads `title.ron` and `title_cinematic.dlg`.
- [ ] Every shot in 0036's storyboard is there, in order; the completion
      notes list each shot's storyboard time and final time.
- [ ] The logo is on screen from the song's last note to the loop.
- [ ] Every snippet line is on Nick's approved list (the notes give the
      source scene of each).
- [ ] Snapshots: one per shot.
- [ ] Watched with sound on native and web for two full loops; the notes
      say so.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Snapshot / integration: one Harness snapshot per shot at a fixed music
  position; the 0819 loop test still passes with the real file.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
