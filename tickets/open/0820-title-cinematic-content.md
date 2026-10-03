---
id: "0820"
title: "The title cinematic itself: shots and snippets timed to the title song"
type: content
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: high
status: todo
blocked_by: ["0036", "0706", "0707", "0803", "0818", "0819", "0827"]
nick_input: sign-off
completed:
---

# 0820 — The title cinematic itself

## Context

Everything needed to play a cinematic on the title exists (0817, 0818,
0819), but `assets/cinematics/title.ron` is a stand-in. This ticket makes
the real one from the storyboard Nick approved in 0036
(`docs/design/title-screen.md`, *Intro cinematic*, **Storyboard**): nine
shots with hard cuts. A pan over the party, the opening lines of
`ch01_intro` in the dialogue screen, six faces sliding past, the opening
lines of `ch01_prebattle`, a march, two close-up fights, the whole field,
and the logo during the song's silence. Its first shot (0:00–0:15) is
hidden by the menu on launch and only seen on later loops, so nothing a
first-time viewer needs goes there.

The title song (`title`, New Sunrise V1) is 133.7 s:

| Time | What the music does |
| ---- | ------------------- |
| 0:00–0:30 | Very quiet opening, slowly rising |
| 0:31–1:24 | Middle section, moderate |
| 1:25–1:56 | Full and loudest |
| 1:57–2:14 | The last note ends, then silence until the loop: **the logo** |

Real content it draws on: the Chapter 1 battle and map (0803), the cast's
bought faces (0706), the Chapter 1 script (0707) and the marching and
fighting shots (0827).

**The snippets are the script itself** (Nick, 0036: "we'll load the same
script file anyway and just play whatever is there for first few lines").
The `Talk` shots point at `ch01_intro` and `ch01_prebattle` in
`assets/dialogue/ch01.dlg`; there is no separate cinematic script and no
list of approved lines. Nick finds the current script too full of quips;
ticket 0724 rewrites it. Either order works: the cinematic shows whatever
the file says. If 0724 lands after this ticket and changes how many boxes
those scenes open with, 0724's session re-checks the counts here.

**No overworld shot yet** (0036, Q4: leave it out). 1010 adds it once the
world map exists.

## Nick input

**Sign-off** after merge, on the Pages build and the Windows download:
watch the title through one whole loop with sound. Does each shot land
where the music changes, is the logo up for the whole pause, do the
snippets give anything away, does anything look off? Comments become
follow-up tickets. Also ask him again, now that he has seen it, whether
Options should get a setting to turn the cinematic off (he said "later"
in 0036; `title-screen.md`, *Open sub-questions*), and record the answer
there (a yes becomes a ticket that adds the row to the Options menu).

## Scope

**In:**
- `assets/cinematics/title.ron`: the storyboard's shots at their times.
- Its two `Talk` shots read `ch01_intro` and `ch01_prebattle` from
  `assets/dialogue/ch01.dlg`, from each scene's first box. No new
  dialogue file.
- The march and the fights as 0827 actions on the Chapter 1 battle.
- Small timing changes so cuts land on the music (keep within a second or
  two of the storyboard; anything bigger goes back to Nick).

**Out (do not do):**
- New shots or characters that aren't in the storyboard, and any change
  to the script's lines. Ask Nick (`ask-nick`) if the storyboard can't be
  built as written.
- Code changes to the player or the title screen. If a shot needs
  something they can't do, write a ticket (`write-ticket`).
- Re-cutting the song.

## Implementation steps

1. Read the storyboard and its rules in `title-screen.md`. Listen to the
   song and note the exact second of each change you want a cut on.
2. Snippets: for each `Talk` shot choose `count`, the number of opening
   boxes of its scene, so that every box can be read twice in the shot's
   time (0818's timing) and the range stops before the scene's first
   `@choice`. Don't pick boxes from the middle of a scene and don't edit
   the script. Check the range against `docs/story/outline.md`: it must
   give away nothing past the opening premise (the openings of these two
   scenes don't today). If a rewrite has made an opening too long to show
   any of it, ask Nick.
3. Write `title.ron`: each shot's `at`, the map pans' start and end tiles
   and zoom on the Chapter 1 battle (`assets/battles/ch01.ron`), the six
   characters in the storyboard's order, the snippets, and `Logo` at the
   song's last note (about 117 s; set it by ear). For the march and the
   fights, write one 0827 action list shared by the four shots: choose
   units and paths that fit the real map and each unit's movement type,
   and fights whose weapons make sense (an archer strikes from two tiles
   away). Nobody in the party falls. Sizes as the storyboard says (3×, 4×,
   4×, 2×).
4. Watch it with the sound on in the native build and the web build, all
   the way round the loop, at least twice. Fix overlaps, cut-off text and
   shots that feel too short to read (a snippet must stay up long enough
   to read twice).
5. Snapshot each shot at one moment.

## Acceptance criteria

- [ ] Content validation loads `title.ron`.
- [ ] Every shot in 0036's storyboard is there, in order; the completion
      notes list each shot's storyboard time and final time.
- [ ] The logo is on screen from the song's last note to the loop.
- [ ] Both snippets start at their scene's first box, stop before its
      first `@choice`, and the notes give each one's scene and box count.
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
