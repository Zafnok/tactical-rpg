---
id: "1010"
title: "Add the overworld shot to the title cinematic"
type: feature
milestone: Post–Chapter 1
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0036", "0820", "1007"]
nick_input: decision
completed:
---

# 1010 — Add the overworld shot to the title cinematic

## Context

Nick's intro cinematic (ticket 0036) shows "a pan/zoom over a battlefield,
over the overworld, past some characters" and conversation snippets. The
world map only arrives after Chapter 1 (1007,
`docs/design/world-structure.md`), so the first version of the cinematic
(0820) has no overworld shot. This ticket adds it once the world map
exists.

0036 decided it this way (Nick, 2026-10-03, on "leave it out for now"):
"3A but make sure to clearly document in 1010 that it updates the
cinematic". So:

**This ticket changes the title cinematic that 0820 shipped.** It edits
`assets/cinematics/title.ron` and the **Storyboard** table in
`docs/design/title-screen.md` (*Intro cinematic*). The decided storyboard
has **no slot** for an overworld shot: its nine shots fill the song. Some
shot has to be shortened, replaced or moved to make room, and that is
Nick's choice.

## Nick input

**Decision first** (`ask-nick`, before building): where the overworld
shot goes and what it takes time from. Show two or three options as
animated mockups against the title song, as 0036 did (an Artifact page; the
game's own renders via `cargo xtask frame-png` if that fits), for example:
it replaces the first shot (0:00–0:15, the pan over the party, which the
menu hides on launch anyway); it takes the second half of the faces shot;
it replaces the whole-field shot before the logo. Say for each what is
lost. Record his answer and the new table in `title-screen.md`.

**Sign-off** after merge: watch the title through one loop. Is the
overworld shot long enough, and does it show anything the player shouldn't
know yet (places or paths from later in the story)?

## Scope

**In:**
- Shot kind `WorldMapPan(map: "<world map id>", from, to, zoom)` in the
  cinematic format and player (0817): the world map drawn as a scene (its
  nodes and paths as a new player would first see them, no army marker, no
  cursor) and shown through 0228's pan and zoom window, like `MapPan`.
- The question to Nick above, and `docs/design/title-screen.md` updated:
  the **Storyboard** table with the new shot and the changed times, the
  "No overworld shot for now" line and the open sub-question removed.
- The shot added to `assets/cinematics/title.ron` at the time Nick chose,
  with the other shots changed exactly as the new table says (0820's
  action list and `Talk` counts re-checked if their shots got shorter).
- `assets/cinematics/README.md` updated.

**Out (do not do):**
- Changes to the world map screen or its data (1007).
- Showing skirmish markers, level markers or anything that depends on a
  save.
- Moving or cutting other shots beyond what Nick chose.
- A zoom between sizes: one whole size per shot (0036).

## Implementation steps

1. Make 1007's world-map drawing callable for a whole map into a
   `GlyphBuffer` (as 0817 did for battle maps), with a fresh campaign's
   view of it: only what is unlocked at the moment the world map first
   opens.
2. Add `WorldMapPan` to the format, the validator (the map exists; `from`
   and `to` are on it; `zoom` is allowed) and the player.
3. Ask Nick where it goes (see *Nick input*); update the storyboard in
   `title-screen.md`.
4. Add the shot to `title.ron` and change the other shots as decided;
   watch the whole loop with sound on native and web.

## Acceptance criteria

- [ ] Each validator error has a test.
- [ ] Snapshot: the overworld shot at its start and end.
- [ ] The shot shows nothing locked at the world map's first opening
      (test: the scene is built from a new campaign).
- [ ] `title-screen.md`'s storyboard table matches `title.ron` shot for
      shot, and no longer says the overworld shot is missing.
- [ ] The other shots' snapshots change only where the new storyboard
      moved them.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: validator.
- Snapshot / integration: the shot through the Harness at fixed music
  positions; the 0819 loop test still passes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
