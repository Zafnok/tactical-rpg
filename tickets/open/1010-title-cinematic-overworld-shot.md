---
id: "1010"
title: "Add the overworld shot to the title cinematic"
type: feature
milestone: Post–Chapter 1
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0036", "0820", "1007"]
nick_input: sign-off
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

If 0036's Q4 chose a map made just for the cinematic instead, 0036 has
replaced or closed this ticket; check `docs/design/title-screen.md`,
*Intro cinematic*, first.

## Nick input

**Sign-off** after merge: watch the title through one loop. Is the
overworld shot in the right place and long enough, and does it show
anything the player shouldn't know yet (places or paths from later in the
story)? The storyboard in `title-screen.md` says where the shot goes; if
it doesn't, ask Nick (`ask-nick`) before building.

## Scope

**In:**
- Shot kind `WorldMapPan(map: "<world map id>", from, to, zoom)` in the
  cinematic format and player (0817): the world map drawn as a scene (its
  nodes and paths as a new player would first see them, no army marker, no
  cursor) and shown through 0228's pan and zoom window, like `MapPan`.
- The shot added to `assets/cinematics/title.ron` at the storyboard's
  time, with the neighbouring shots' times adjusted as the storyboard
  says.
- `assets/cinematics/README.md` updated.

**Out (do not do):**
- Changes to the world map screen or its data (1007).
- Showing skirmish markers, level markers or anything that depends on a
  save.
- Moving or cutting other shots beyond what the storyboard says.

## Implementation steps

1. Make 1007's world-map drawing callable for a whole map into a
   `GlyphBuffer` (as 0817 did for battle maps), with a fresh campaign's
   view of it: only what is unlocked at the moment the world map first
   opens.
2. Add `WorldMapPan` to the format, the validator (the map exists; `from`
   and `to` are on it; `zoom` is allowed) and the player.
3. Add the shot to `title.ron`; watch the whole loop with sound on native
   and web.

## Acceptance criteria

- [ ] Each validator error has a test.
- [ ] Snapshot: the overworld shot at its start and end.
- [ ] The shot shows nothing locked at the world map's first opening
      (test: the scene is built from a new campaign).
- [ ] The other shots' snapshots change only where the storyboard moved
      them.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: validator.
- Snapshot / integration: the shot through the Harness at fixed music
  positions; the 0819 loop test still passes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
